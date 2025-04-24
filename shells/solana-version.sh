#!/bin/bash

# 这个脚本用于检查Solana环境和工具链版本

echo "===== Solana环境检查 ====="

# 检查Solana CLI版本
echo "Solana CLI版本:"
solana --version

# 检查Anchor版本
echo -e "\nAnchor版本:"
anchor --version

# 检查Rust版本
echo -e "\nRust版本:"
rustc --version
cargo --version

# 检查节点连接
echo -e "\n检查DevNet连接:"
solana cluster-version --url https://api.devnet.solana.com

# 检查部署账户
echo -e "\n检查部署账户余额:"
KEYPAIR="./deploy-keypair.json"
if [ -f "$KEYPAIR" ]; then
  PUBKEY=$(solana-keygen pubkey $KEYPAIR)
  echo "部署公钥: $PUBKEY"
  solana balance $PUBKEY --url https://api.devnet.solana.com
else
  echo "错误: 部署密钥对文件不存在: $KEYPAIR"
fi

# 检查程序ID是否已被使用
echo -e "\n检查程序ID状态:"
PROGRAM_ID="F8bssPyuN6H4HoJtWRW4koomPiLnr8hgAeQaQW9yiRER"
solana program show $PROGRAM_ID --url https://api.devnet.solana.com || echo "程序ID未被使用或不可访问"

# 检查程序大小
echo -e "\n检查程序大小:"
PROGRAM_SO="target/deploy/wusd_token.so"
if [ -f "$PROGRAM_SO" ]; then
  ls -lh $PROGRAM_SO
  echo "程序大小: $(ls -lh $PROGRAM_SO | awk '{print $5}')"
else
  echo "程序未编译: $PROGRAM_SO"
fi

echo -e "\n===== 环境检查完成 =====" 