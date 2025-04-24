import * as anchor from "@coral-xyz/anchor";
import * as spl from "@solana/spl-token";
import { TOKEN_2022_PROGRAM_ID } from "@solana/spl-token";

// 最小SOL金额
export const MIN_SOL_BALANCE = 0.3; // 0.3 SOL

// 转账SOL给指定账户
export async function fundAccount(
  connection: anchor.web3.Connection,
  from: anchor.web3.Keypair,
  to: anchor.web3.PublicKey,
  amountSol: number = 0.1
): Promise<string> {
  const tx = new anchor.web3.Transaction().add(
    anchor.web3.SystemProgram.transfer({
      fromPubkey: from.publicKey,
      toPubkey: to,
      lamports: amountSol * anchor.web3.LAMPORTS_PER_SOL,
    })
  );
  
  return await connection.sendTransaction(tx, [from]);
}

// 检查账户余额
export async function checkAccountBalance(
  connection: anchor.web3.Connection,
  account: anchor.web3.PublicKey,
  minimumSol: number = MIN_SOL_BALANCE
): Promise<boolean> {
  const balance = await connection.getBalance(account);
  const minimumLamports = minimumSol * anchor.web3.LAMPORTS_PER_SOL;
  return balance >= minimumLamports;
}

// 按需转账SOL
export async function ensureAccountBalance(
  connection: anchor.web3.Connection,
  from: anchor.web3.Keypair,
  to: anchor.web3.PublicKey,
  targetSol: number = 0.1
): Promise<void> {
  const currentBalance = await connection.getBalance(to);
  const targetLamports = targetSol * anchor.web3.LAMPORTS_PER_SOL;
  
  if (currentBalance < targetLamports) {
    const transferAmount = targetLamports - currentBalance;  
    await fundAccount(
      connection,
      from,
      to,
      transferAmount / anchor.web3.LAMPORTS_PER_SOL
    );
  }
}

// 创建Token账户
export async function createTokenAccount(
  connection: anchor.web3.Connection, 
  payer: anchor.web3.Keypair,
  mint: anchor.web3.PublicKey,
  owner: anchor.web3.PublicKey
): Promise<anchor.web3.PublicKey> {
  // 确保支付账户有足够的SOL
  await ensureAccountBalance(connection, payer, payer.publicKey);
  
  try {
    // 检查是否已经存在关联账户
    const associatedToken = await spl.getAssociatedTokenAddress(
      mint,
      owner,
      true,
      TOKEN_2022_PROGRAM_ID
    );
    
    try {
      // 尝试获取账户信息，看是否已经存在
      await spl.getAccount(
        connection,
        associatedToken,
        "confirmed",
        TOKEN_2022_PROGRAM_ID
      ); 
      return associatedToken;
    } catch (e) {
      // 账户不存在，创建新账户
      console.log(`创建新的关联Token账户: ${associatedToken.toBase58()}`);
      return await spl.createAssociatedTokenAccount(
        connection,
        payer,
        mint,
        owner,
        { commitment: "confirmed" },
        TOKEN_2022_PROGRAM_ID
      );
    }
  } catch (e) {
    // 无法使用关联账户，创建普通账户
    console.log(`创建普通Token账户`);
    return await spl.createAccount(
      connection,
      payer,
      mint,
      owner,
      undefined,
      { commitment: "confirmed" },
      TOKEN_2022_PROGRAM_ID
    );
  }
}

// 等待交易确认
export async function waitForConfirmation(
  connection: anchor.web3.Connection,
  signature: string,
  confirmationLevel: anchor.web3.Finality = "confirmed"
): Promise<void> {
  await connection.confirmTransaction(signature, confirmationLevel);
} 