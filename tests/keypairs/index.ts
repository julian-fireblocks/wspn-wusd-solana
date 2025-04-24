import * as anchor from "@coral-xyz/anchor";
import fs from "fs";
import path from "path";

// 密钥管理器类
export class KeypairManager {
  private keypairs: Map<string, anchor.web3.Keypair> = new Map();
  private readonly keypairsDir: string;
  
  constructor(keypairsDir: string = path.join(__dirname)) {
    this.keypairsDir = keypairsDir;
    
    // 创建目录（如果不存在）
    if (!fs.existsSync(this.keypairsDir)) {
      fs.mkdirSync(this.keypairsDir, { recursive: true });
    }
  }
  
  // 获取或创建密钥对
  public getOrCreate(name: string): anchor.web3.Keypair {
    // 检查是否已加载
    if (this.keypairs.has(name)) {
      return this.keypairs.get(name)!;
    }
    
    const keypairPath = path.join(this.keypairsDir, `${name}.json`);
    
    // 检查是否存在密钥文件
    if (fs.existsSync(keypairPath)) {
      try {
        // 从文件加载密钥
        const keypairData = JSON.parse(fs.readFileSync(keypairPath, 'utf-8'));
        const keypair = anchor.web3.Keypair.fromSecretKey(
          new Uint8Array(keypairData)
        );
        this.keypairs.set(name, keypair);
        return keypair;
      } catch (error) {
        console.warn(`Error loading keypair ${name}:`, error);
        // 如果加载失败，创建新密钥
      }
    }
    
    // 创建新密钥对
    const keypair = anchor.web3.Keypair.generate();
    
    // 保存到文件
    fs.writeFileSync(
      keypairPath, 
      JSON.stringify(Array.from(keypair.secretKey)),
      'utf-8'
    );
    
    // 保存到内存
    this.keypairs.set(name, keypair);
    return keypair;
  }
  
  // 检查账户是否有足够余额
  public async checkBalance(
    connection: anchor.web3.Connection,
    keypair: anchor.web3.Keypair,
    minBalanceSol: number = 0.1
  ): Promise<boolean> {
    const balance = await connection.getBalance(keypair.publicKey);
    return balance >= minBalanceSol * anchor.web3.LAMPORTS_PER_SOL;
  }
  
  // 确保账户有足够余额
  public async ensureBalance(
    connection: anchor.web3.Connection,
    keypair: anchor.web3.Keypair,
    funder: anchor.web3.Keypair,
    targetBalanceSol: number = 0.2
  ): Promise<void> {
    const targetBalance = targetBalanceSol * anchor.web3.LAMPORTS_PER_SOL;
    const currentBalance = await connection.getBalance(keypair.publicKey);
    
    if (currentBalance < targetBalance) {
      const neededAmount = targetBalance - currentBalance;
      
      // 转账交易
      const tx = new anchor.web3.Transaction().add(
        anchor.web3.SystemProgram.transfer({
          fromPubkey: funder.publicKey,
          toPubkey: keypair.publicKey,
          lamports: neededAmount,
        })
      );
      
      // 发送交易
      await connection.sendTransaction(tx, [funder]);
      
      // 等待确认
      await new Promise(resolve => setTimeout(resolve, 1000));
      console.log(`Funded ${keypair.publicKey.toString()} with ${neededAmount / anchor.web3.LAMPORTS_PER_SOL} SOL`);
    }
  }
  
  // 清理不需要的余额
  public async recoverFunds(
    connection: anchor.web3.Connection,
    keypair: anchor.web3.Keypair,
    recipient: anchor.web3.PublicKey,
    reserveSol: number = 0.01
  ): Promise<void> {
    const reserveAmount = reserveSol * anchor.web3.LAMPORTS_PER_SOL;
    const currentBalance = await connection.getBalance(keypair.publicKey);
    
    if (currentBalance > reserveAmount) {
      const transferAmount = currentBalance - reserveAmount - 5000; // 减去保留金额和交易费用
      
      if (transferAmount > 0) {
        // 转账交易
        const tx = new anchor.web3.Transaction().add(
          anchor.web3.SystemProgram.transfer({
            fromPubkey: keypair.publicKey,
            toPubkey: recipient,
            lamports: transferAmount,
          })
        );
        
        // 发送交易
        await connection.sendTransaction(tx, [keypair]);
        
        // 等待确认
        await new Promise(resolve => setTimeout(resolve, 1000));
        console.log(`Recovered ${transferAmount / anchor.web3.LAMPORTS_PER_SOL} SOL from ${keypair.publicKey.toString()}`);
      }
    }
  }
}

// 导出单例实例
export const keypairManager = new KeypairManager(); 