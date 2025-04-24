#!/bin/bash

# 这个脚本使用拆分流程部署，先创建缓冲区，然后部署程序

PROGRAM_ID="F8bssPyuN6H4HoJtWRW4koomPiLnr8hgAeQaQW9yiRER"
PROGRAM_SO="target/deploy/wusd_token.so"
KEYPAIR="./deploy-keypair.json"
URL="https://api.devnet.solana.com"

echo "===== 开始拆分部署流程 ====="

# 步骤1：创建缓冲区密钥对
echo "步骤1：创建专用缓冲区密钥对..."
BUFFER_KEYPAIR="buffer-keypair.json"
solana-keygen new --no-bip39-passphrase --force --outfile $BUFFER_KEYPAIR
if [ $? -ne 0 ]; then
  echo "创建缓冲区密钥对失败，退出"
  exit 1
fi
echo "缓冲区密钥对已创建: $BUFFER_KEYPAIR"

# 步骤2：向缓冲区密钥对发送SOL
echo "步骤2：向缓冲区账户发送SOL..."
BUFFER_PUBKEY=$(solana-keygen pubkey $BUFFER_KEYPAIR)
echo "缓冲区公钥: $BUFFER_PUBKEY"
solana transfer --from $KEYPAIR $BUFFER_PUBKEY 0.5 --allow-unfunded-recipient --url $URL
if [ $? -ne 0 ]; then
  echo "向缓冲区转账失败，退出"
  exit 1
fi

# 步骤3：写入缓冲区
echo "步骤3：写入程序到缓冲区..."
solana program write-buffer $PROGRAM_SO --buffer $BUFFER_KEYPAIR --url $URL
if [ $? -ne 0 ]; then
  echo "写入缓冲区失败，退出"
  exit 1
fi

# 步骤4：设置缓冲区权限
echo "步骤4：设置缓冲区权限..."
DEPLOYER_PUBKEY=$(solana-keygen pubkey $KEYPAIR)
solana program set-buffer-authority --new-buffer-authority $DEPLOYER_PUBKEY $BUFFER_PUBKEY --buffer-authority $BUFFER_KEYPAIR --url $URL
if [ $? -ne 0 ]; then
  echo "设置缓冲区权限失败，退出"
  exit 1
fi

# 步骤5：部署程序
echo "步骤5：从缓冲区部署程序..."
solana program deploy --program-id $PROGRAM_ID --buffer $BUFFER_PUBKEY --keypair $KEYPAIR --url $URL
DEPLOY_RESULT=$?

# 清理
echo "清理临时文件..."
rm -f $BUFFER_KEYPAIR

# 检查结果
if [ $DEPLOY_RESULT -eq 0 ]; then
  echo "===== 部署成功！ ====="
  solana program show $PROGRAM_ID --url $URL
else
  echo "===== 部署失败！ ====="
  echo "请检查错误信息并尝试手动部署。"
fi

exit $DEPLOY_RESULT 