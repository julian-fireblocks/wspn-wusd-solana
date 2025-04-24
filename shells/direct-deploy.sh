#!/bin/bash

# 这是一个简单的部署脚本，使用直接命令而不依赖复杂逻辑

echo "开始简单部署过程..."

# 1. 编译程序（如果需要）
anchor build

# 2. 检查部署账户余额
solana balance --keypair ./deploy-keypair.json --url https://api.devnet.solana.com

# 3. 直接部署程序
solana program deploy \
  target/deploy/wusd_token.so \
  --program-id F8bssPyuN6H4HoJtWRW4koomPiLnr8hgAeQaQW9yiRER \
  --keypair ./deploy-keypair.json \
  --url https://api.devnet.solana.com

# 4. 显示部署结果
if [ $? -eq 0 ]; then
  echo "部署成功！"
  solana program show F8bssPyuN6H4HoJtWRW4koomPiLnr8hgAeQaQW9yiRER --url https://api.devnet.solana.com
else
  echo "部署失败。"
fi 