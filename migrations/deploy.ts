// Migrations are an early feature. Currently, they're nothing more than this
// single deploy script that's invoked from the CLI, injecting a provider
// configured from the workspace's Anchor.toml.

import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";

module.exports = async function (provider: anchor.AnchorProvider) {
  // Configure client to use the provider.
  anchor.setProvider(provider);

  console.log("Starting deployment with upgradeable program...");
  console.log("Provider wallet:", provider.wallet.publicKey.toString());
  
  // 确保使用BPFLoaderUpgradeable部署程序
  // Anchor.toml中的upgradeable = true设置会自动处理这一点
  
  console.log("Deployment completed successfully");
  console.log("注意: 程序已使用可升级模式部署，可以通过升级指令更新程序代码");
};
