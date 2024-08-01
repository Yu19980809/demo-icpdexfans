import { config } from 'dotenv'

const envList = config({ path: './.env' }).parsed || {}
envList.NEXT_PUBLIC_IC_HOST =
  envList.DFX_NETWORK === "ic" ? "https://icp-api.io" : "http://localhost:4943"

/** @type {import('next').NextConfig} */
const nextConfig = {
  env: envList,
  output: 'export'
};

export default nextConfig;
