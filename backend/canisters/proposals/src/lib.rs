use candid::{CandidType, Decode, Encode, Principal};
use ic_cdk::caller;
use ic_cdk_macros::*;
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use ic_stable_structures::{BoundedStorable, Cell, DefaultMemoryImpl, StableBTreeMap, Storable};
use serde::Deserialize;
use std::{borrow::Cow, cell::RefCell};

type Memory = VirtualMemory<DefaultMemoryImpl>;
type IdCell = Cell<u64, Memory>;

#[derive(CandidType, Deserialize)]
enum Choice {
    Approve,
    Reject,
    Pass
}

#[derive(CandidType)]
enum VoteError {
    AlreadyVoted,
    ProposalIsNotActive,
    InvalidChoice,
    NoSuchProposal,
    AccessRejected,
    UpdateError
}

#[derive(CandidType, Clone, Deserialize)]
struct Proposal {
    description: String,
    approve: u32,
    reject: u32,
    pass: u32,
    is_active: bool,
    voted: Vec<Principal>,
    owner: Principal
}

#[derive(CandidType, Deserialize)]
struct ProposalPayload {
    description: String,
    is_active: bool
}

impl Storable for Proposal {
    fn to_bytes(&self) -> Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
}

impl BoundedStorable for Proposal {
    const MAX_SIZE: u32 = 5000;
    const IS_FIXED_SIZE: bool = false;
}

thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> = RefCell::new(
        MemoryManager::init(DefaultMemoryImpl::default())
    );

    static ID_COUNTER: RefCell<IdCell> = RefCell::new(
        IdCell::init(MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(0))), 0)
            .expect("Cannot create a counter")
    );

    static STORAGE: RefCell<StableBTreeMap<u64, Proposal, Memory>> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(1)))
        )
    );
}

#[query]
fn get_proposal_count() -> u64 {
    STORAGE.with(|s| s.borrow().len())
}

#[update]
fn create_proposal(id: u64, payload: ProposalPayload) -> Option<Proposal> {
    let proposal = Proposal {
        description: payload.description,
        approve: 0_u32,
        reject: 0_u32,
        pass: 0_u32,
        is_active: payload.is_active,
        voted: vec![],
        owner: caller()
    };

    _insert_proposal(id, &proposal);
    Some(proposal)
}

fn _insert_proposal(id: u64, proposal: &Proposal) {
    STORAGE.with(|s| s.borrow_mut().insert(id, proposal.clone()));
}

#[update]
fn edit_proposal(id: u64, payload: ProposalPayload) -> Result<(), VoteError> {
    STORAGE.with(|s| {
        let old_proposal_temp = s.borrow().get(&id);
        let old_proposal;

        match old_proposal_temp {
            Some(value) => old_proposal = value,
            None => return Err(VoteError::NoSuchProposal)
        }

        if caller() != old_proposal.owner {
            return Err(VoteError::AccessRejected)
        }

        let value = Proposal {
            description: payload.description,
            is_active: payload.is_active,
            approve: old_proposal.approve,
            reject: old_proposal.reject,
            pass: old_proposal.pass,
            voted: old_proposal.voted,
            owner: caller()
        };

        let res = s.borrow_mut().insert(id, value);

        match res {
            Some(_) => Ok(()),
            None => Err(VoteError::UpdateError)
        }
    })
}

#[update]
fn end_proposal(id: u64) -> Result<(), VoteError> {
    STORAGE.with(|s| {
        let proposal_temp = s.borrow().get(&id);
        let mut proposal;

        match proposal_temp {
            Some(value) => proposal = value,
            None => return Err(VoteError::NoSuchProposal)
        }

        if caller() != proposal.owner {
            return Err(VoteError::AccessRejected)
        }

        proposal.is_active = false;

        let res = s.borrow_mut().insert(id, proposal);

        match res {
            Some(_) => Ok(()),
            None => Err(VoteError::UpdateError)
        }
    })
}

#[update]
fn vote(id: u64, choice: Choice) -> Result<(), VoteError> {
    STORAGE.with(|s| {
        let proposal_temp = s.borrow().get(&id);
        let mut proposal;

        match proposal_temp {
            Some(value) => proposal = value,
            None => return Err(VoteError::NoSuchProposal)
        }

        let caller = caller();

        if proposal.is_active == false {
            return Err(VoteError::ProposalIsNotActive)
        } else if proposal.voted.contains(&caller) {
            return Err(VoteError::AlreadyVoted)
        }

        match choice {
            Choice::Approve => proposal.approve += 1,
            Choice::Reject => proposal.reject += 1,
            Choice::Pass => proposal.pass += 1
        }

        proposal.voted.push(caller);

        let res = s.borrow_mut().insert(id, proposal);

        match res {
            Some(_) => Ok(()),
            None => Err(VoteError::UpdateError)
        }
    })
}
