#!/bin/bash
# 设置颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${YELLOW}开始WUSD代币程序部署到DevNet的流程...${NC}"

# 提供RPC节点选项
PRIMARY_RPC="https://api.devnet.solana.com"
CURRENT_RPC=$PRIMARY_RPC

# 确保环境变量正确设置
export ANCHOR_PROVIDER_URL=$CURRENT_RPC
export ANCHOR_WALLET="./deploy-keypair.json"

# 设置程序ID和文件路径
PROGRAM_KEYPAIR="target/deploy/wusd_token-keypair.json"
# 从program keypair文件中获取程序ID
if [ -f "$PROGRAM_KEYPAIR" ]; then
  PROGRAM_ID=$(solana-keygen pubkey $PROGRAM_KEYPAIR)
  echo -e "${GREEN}从密钥对获取的程序ID: $PROGRAM_ID${NC}"
else
  PROGRAM_ID="HrkSbtcCVsWJ1dpRKS7rNgwrga74XR7izJWn59BVWKbG"
  echo -e "${YELLOW}使用手动设置的程序ID: $PROGRAM_ID${NC}"
fi

PROGRAM_SO="target/deploy/wusd_token.so"
DEPLOY_KEYPAIR="./deploy-keypair.json"

# 步骤1：检查账户余额
echo -e "${YELLOW}步骤1: 检查部署账户余额${NC}"
BALANCE=$(solana balance --keypair $DEPLOY_KEYPAIR --url $CURRENT_RPC)
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

# 获取程序大小
PROGRAM_SIZE=$(du -h $PROGRAM_SO | cut -f1)
echo -e "${YELLOW}程序大小: ${PROGRAM_SIZE}${NC}"

# 检查Solana版本
echo -e "${YELLOW}检查Solana CLI版本...${NC}"
SOLANA_VERSION=$(solana --version)
echo -e "${GREEN}$SOLANA_VERSION${NC}"

# 检查程序ID是否已存在
echo -e "${YELLOW}检查程序ID是否已部署...${NC}"
PROGRAM_INFO=$(solana program show $PROGRAM_ID --url $CURRENT_RPC 2>/dev/null)
PROGRAM_EXISTS=$?

if [ $PROGRAM_EXISTS -eq 0 ]; then
  echo -e "${YELLOW}该程序ID已经部署过，将执行升级操作${NC}"
  UPGRADE_MODE=true
else
  echo -e "${YELLOW}该程序ID尚未部署，将执行新部署操作${NC}"
  UPGRADE_MODE=false
fi

# 步骤3：部署尝试
echo -e "${YELLOW}步骤3: 开始部署程序${NC}" 

# 如果是新部署，尝试使用program keypair文件
if [ "$UPGRADE_MODE" = false ]; then
  echo -e "${YELLOW}尝试新程序部署方法...${NC}" 
  anchor deploy \
    --provider.cluster $CURRENT_RPC \
    --program-name wusd_token \
  
  DEPLOY_RESULT=$?
  
  if [ $DEPLOY_RESULT -eq 0 ]; then
    echo -e "${GREEN}部署成功！${NC}"
  else
    echo -e "${YELLOW}部署失败!"  
    # 诊断信息
    echo -e "${YELLOW}详细诊断信息:${NC}"
    echo "1. Solana CLI版本: $SOLANA_VERSION"
    echo "2. 检查DevNet状态: https://status.solana.com/"
    echo "3. 程序大小: $PROGRAM_SIZE (${SIZE_BYTES} 字节)"
    echo "4. 当前余额: $BALANCE (确保有足够的SOL)"
    echo "5. 程序权限信息:"
    echo "$PROGRAM_INFO" 
    exit 1
  fi
fi

# 验证程序部署
echo -e "${YELLOW}验证部署...${NC}"
DEPLOYED_PROGRAM=$(solana program show $PROGRAM_ID --url $CURRENT_RPC)
echo "$DEPLOYED_PROGRAM"

echo -e "${GREEN}部署流程完成！${NC}"
exit 0 