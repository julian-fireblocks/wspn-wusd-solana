#!/bin/bash

# 设置颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${YELLOW}开始WUSD代币程序部署到DevNet的替代流程...${NC}"

# 确保环境变量正确设置
export ANCHOR_PROVIDER_URL="https://api.devnet.solana.com"
export ANCHOR_WALLET="./deploy-keypair.json"

# 设置程序ID和文件路径
PROGRAM_ID="F8bssPyuN6H4HoJtWRW4koomPiLnr8hgAeQaQW9yiRER"
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

# 步骤3: 直接使用替代方法尝试部署
echo -e "${YELLOW}步骤3: 尝试使用替代RPC节点部署${NC}"

# 使用不同RPC节点列表
RPC_NODES=(
  "https://api.devnet.solana.com"
  "https://devnet.genesysgo.net"
  "https://devnet.helius-rpc.com"
)

# 尝试每个RPC节点
for RPC_NODE in "${RPC_NODES[@]}"; do
  echo -e "${YELLOW}尝试使用RPC节点: $RPC_NODE${NC}"
  
  # 设置较低的分块大小，提高稳定性
  CHUNK_SIZE="--max-len 1000" 
  
  echo -e "${YELLOW}使用较小分块大小部署...${NC}"
  solana program deploy \
    $PROGRAM_SO \
    --program-id $PROGRAM_ID \
    --keypair $KEYPAIR \
    --url $RPC_NODE \
    --commitment finalized \
    $CHUNK_SIZE
  
  DEPLOY_RESULT=$?
  
  if [ $DEPLOY_RESULT -eq 0 ]; then
    echo -e "${GREEN}部署成功！${NC}"
    echo -e "程序ID: ${GREEN}$PROGRAM_ID${NC}"
    echo -e "使用的RPC节点: ${GREEN}$RPC_NODE${NC}"
    
    # 验证程序部署
    echo -e "${YELLOW}验证部署...${NC}"
    DEPLOYED_PROGRAM=$(solana program show $PROGRAM_ID --url $RPC_NODE)
    echo "$DEPLOYED_PROGRAM"
    
    # 保存成功的RPC节点到配置
    solana config set --url $RPC_NODE
    
    exit 0
  else
    echo -e "${RED}使用 $RPC_NODE 部署失败，尝试下一个节点...${NC}"
  fi
done

# 所有RPC节点均失败，尝试使用Anchor部署
echo -e "${YELLOW}所有RPC节点部署尝试均失败，尝试使用Anchor部署...${NC}"

ANCHOR_URL="https://api.devnet.solana.com"
echo -e "${YELLOW}使用Anchor部署到 $ANCHOR_URL...${NC}"

anchor deploy \
  --provider.cluster $ANCHOR_URL \
  --program-name wusd_token \
  --program-id $PROGRAM_ID

ANCHOR_RESULT=$?

if [ $ANCHOR_RESULT -eq 0 ]; then
  echo -e "${GREEN}使用Anchor部署成功！${NC}"
  exit 0
else
  # 最后尝试方法 - 使用较大的超时时间并使用默认RPC节点
  echo -e "${YELLOW}尝试最终方法 - 增加超时时间...${NC}"
  
  # 增加超时时间和重试次数
  export SOLANA_TIMEOUT_SECONDS=600  # 设置10分钟超时
  
  solana program deploy \
    $PROGRAM_SO \
    --program-id $PROGRAM_ID \
    --keypair $KEYPAIR \
    --url https://api.devnet.solana.com
  
  FINAL_RESULT=$?
  
  if [ $FINAL_RESULT -eq 0 ]; then
    echo -e "${GREEN}使用最终方法部署成功！${NC}"
    exit 0
  else
    echo -e "${RED}所有部署方法都失败！${NC}"
    echo -e "${YELLOW}详细诊断：${NC}"
    echo "1. 程序大小: $(ls -lh $PROGRAM_SO | awk '{print $5}')"
    echo "2. 账户余额: $BALANCE"
    echo "3. 可能需要手动通过Solana CLI部署"
    echo -e "${YELLOW}建议手动执行以下命令：${NC}"
    echo "solana program deploy $PROGRAM_SO --program-id $PROGRAM_ID --keypair $KEYPAIR"
    exit 1
  fi
fi 