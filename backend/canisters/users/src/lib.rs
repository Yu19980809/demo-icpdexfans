use candid::{CandidType, Decode, Encode, Principal};
use ic_cdk::api::time;
use ic_cdk::caller;
use ic_cdk_macros::*;
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use ic_stable_structures::{BoundedStorable, Cell, DefaultMemoryImpl, StableBTreeMap, Storable};
use std::{borrow::Cow, cell::RefCell};
use serde::{Deserialize, Serialize};

type Memory = VirtualMemory<DefaultMemoryImpl>;
type IdCell = Cell<u64, Memory>;

#[derive(CandidType, Clone, Serialize, Deserialize)]
enum PostType {
    Free,
    Silver,
    Gold,
    Platinum,
    Paid
}

#[derive(CandidType, Serialize, Deserialize)]
enum PostError {
    PostNotFound,
    AlreadyLiked,
}

#[derive(CandidType, Clone, Serialize, Deserialize)]
struct Post {
    id: u64,
    content: String,
    image: Option<String>,
    video: Option<String>,
    post_type: PostType,
    creator_id: Principal,
    likes: Vec<Principal>,
    comments: Vec<String>,
    created_at: u64,
    updated_at: Option<u64>
}

#[derive(CandidType, Serialize, Deserialize)]
struct PostPayload {
    content: String,
    image: Option<String>,
    video: Option<String>,
    post_type: PostType,
}

impl Storable for Post {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
}

impl BoundedStorable for Post {
    const MAX_SIZE: u32 = 1024;
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

    static POST_MAP: RefCell<StableBTreeMap<u64, Post, Memory>> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(1)))
        )
    );
}

#[query]
fn get_post_detail(id: u64) -> Result<Post, PostError> {
    let post = POST_MAP.with(|map| map.borrow().get(&id));
    match post {
        Some(value) => Ok(value),
        None => Err(PostError::PostNotFound)
    }
}

#[update]
fn publish_post(payload: PostPayload) -> Option<Post> {
    let id = ID_COUNTER
        .with(|counter| {
            let current_value = *counter.borrow().get();
            counter.borrow_mut().set(current_value + 1)
        })
        .expect("Cannot increment id counter");

    let post = Post {
        id,
        content: payload.content,
        image: payload.image,
        video: payload.video,
        post_type: payload.post_type,
        creator_id: caller(),
        likes: vec![],
        comments: vec![],
        created_at: time(),
        updated_at: None
    };

    _insert_post(&post);
    Some(post)
}

fn _insert_post(post: &Post) {
    POST_MAP.with(|map| map.borrow_mut().insert(post.id, post.clone()));
}
