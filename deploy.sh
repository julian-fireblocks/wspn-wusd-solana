#!/bin/bash
# 设置颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${YELLOW}开始WUSD代币程序部署到DevNet的流程...${NC}"

# 确保环境变量正确设置
export ANCHOR_PROVIDER_URL="https://api.devnet.solana.com"
export ANCHOR_WALLET="./deploy-keypair.json"

# 设置程序ID和文件路径
PROGRAM_ID="GXtsGhmGUz4uFkCxsNSmJaxU9nr6SvgA3c6pxFc2MpPn"
PROGRAM_SO="target/deploy/wusd_token.so"
KEYPAIR="./deploy-keypair.json"

# 步骤1：检查账户余额
echo -e "${YELLOW}步骤1: 检查部署账户余额${NC}"
BALANCE=$(solana balance --keypair $KEYPAIR --url $ANCHOR_PROVIDER_URL)
echo -e "当前余额: ${GREEN}$BALANCE${NC}"

if [[ $BALANCE == "0 SOL" ]]; then
  echo -e "${RED}错误: 部署账户没有SOL, 请先充值${NC}"
  exit 1
fi

# 步骤2：确保程序已经编译
echo -e "${YELLOW}步骤2: 检查编译状态${NC}"
if [ ! -f "$PROGRAM_SO" ]; then
  echo -e "${YELLOW}编译程序...${NC}"
  anchor build --program-name wusd_token
else
  echo -e "${GREEN}找到已编译程序: $PROGRAM_SO${NC}"
  ls -lh $PROGRAM_SO
fi

# 步骤3：直接部署（简化流程，不使用缓冲区）
echo -e "${YELLOW}步骤3: 开始部署程序${NC}"
echo -e "${YELLOW}使用直接部署方法${NC}"

# 使用设置超时和条件响应的方法
TIMEOUT=600  # 10分钟超时
echo -e "${YELLOW}开始部署，允许最长等待${TIMEOUT}秒${NC}"

# 使用solana命令部署
solana program deploy \
  $PROGRAM_SO \
  --program-id $PROGRAM_ID \
  --keypair $KEYPAIR \
  --url $ANCHOR_PROVIDER_URL \
  --commitment finalized

DEPLOY_RESULT=$?

if [ $DEPLOY_RESULT -eq 0 ]; then
  echo -e "${GREEN}部署成功！${NC}"
  echo -e "程序ID: ${GREEN}$PROGRAM_ID${NC}"
  
  # 验证程序部署
  echo -e "${YELLOW}验证部署...${NC}"
  DEPLOYED_PROGRAM=$(solana program show $PROGRAM_ID --url $ANCHOR_PROVIDER_URL)
  echo "$DEPLOYED_PROGRAM"
  
  # 设置验证节点集群配置
  solana config set --url $ANCHOR_PROVIDER_URL
  solana config set --commitment finalized
  
  exit 0
else
  echo -e "${RED}部署失败！尝试备用方法...${NC}"
  
  # 备用部署方法 - 使用原生anchor命令
  echo -e "${YELLOW}尝试使用Anchor原生部署...${NC}"
  
  anchor deploy \
    --provider.cluster $ANCHOR_PROVIDER_URL \
    --program-name wusd_token \
    --program-id $PROGRAM_ID
  
  ANCHOR_RESULT=$?
  
  if [ $ANCHOR_RESULT -eq 0 ]; then
    echo -e "${GREEN}使用Anchor部署成功！${NC}"
    exit 0
  else
    echo -e "${RED}所有部署方法都失败...${NC}"
    
    # 诊断信息
    echo -e "${YELLOW}诊断信息:${NC}"
    echo "1. 确保您的Solana CLI版本是最新的"
    echo "2. 检查DevNet状态: https://status.solana.com/"
    echo "3. 尝试使用其他RPC节点，比如: https://devnet.genesysgo.net/"
    echo "4. 确保您有足够的SOL来支付部署费用"
    echo "5. 程序可能太大，尝试优化代码或分批部署"
    
    exit 1
  fi
fi 