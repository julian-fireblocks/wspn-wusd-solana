import {
  Keypair,
  PublicKey,
  sendAndConfirmTransaction,
  Transaction,
  clusterApiUrl,
  SystemProgram,
  TransactionInstruction,
} from "@solana/web3.js";
import {
  FireblocksConnectionAdapter,
  FireblocksConnectionAdapterConfig,
  FeeLevel,
} from "./fireblocks/index";
import { TOKEN_2022_PROGRAM_ID } from "@solana/spl-token";
import BN from "bn.js";
import * as anchor from "@coral-xyz/anchor";
import * as fs from "fs";

require("dotenv").config();

const mintToken = async () => {
  let program: any; 
  let programId: anchor.web3.PublicKey;
  const amount = new BN(10000000000000); // 10000000 WUSD (考虑到6位小数)
  let recipientTokenAccount = new PublicKey(
    process.env.WUSD_RECIPIENT_ADDRESS || ""
  );
  console.log("Starting token minting process...");

  // 配置Fireblocks连接
  const fireblocksConnectionConfig: FireblocksConnectionAdapterConfig = {
    apiKey: process.env.FIREBLOCKS_API_KEY || "",
    apiSecretPath: process.env.FIREBLOCKS_SECRET_KEY_PATH || "",
    vaultAccountId: process.env.MINTER_VAULT_ACCOUNT_ID || "",
    feeLevel: FeeLevel.HIGH, // 使用高费用级别进行铸币交易
    silent: false,
    devnet: true, // 设置为true表示devnet，false表示mainnet
  };

  // 创建到Solana devnet的连接
  const connection = await FireblocksConnectionAdapter.create(
    clusterApiUrl("devnet"),
    fireblocksConnectionConfig
  );

  // 获取铸币者的公钥
  const minter = new PublicKey(connection.getAccount());
  console.log("Minter account:", minter.toBase58());
  console.log("Recipient token account:", recipientTokenAccount.toBase58());

  // 获取代币铸造账户
  let tokenMintKeypair;
  try {
    // 尝试从文件中加载密钥对
    const keypairFile = process.env.WUSD_TOKENMINT_KEYPAIR_PATH || "";
    if (keypairFile) {
      const keypairData = fs.readFileSync(keypairFile, "utf8");
      tokenMintKeypair = Keypair.fromSecretKey(
        Buffer.from(JSON.parse(keypairData))
      );
    } else {
      throw new Error("No keypair file path provided");
    }
  } catch (error) {
    console.error("Failed to load tokenMint keypair:", error);
    throw error;
  }

  const tokenMint = tokenMintKeypair.publicKey;
  console.log("Token mint:", tokenMint.toBase58());
  
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  try {
    // 首先尝试使用workspace获取程序
    program = anchor.workspace.WusdToken;
    programId = program.programId;
  } catch (e) {
    // 如果失败，从构建的keypair文件获取
    console.log("无法从workspace获取程序，尝试从keypair文件读取...");
    const programKeypair = require("../target/deploy/wusd_token-keypair.json");
    programId = anchor.web3.Keypair.fromSecretKey(
      new Uint8Array(programKeypair)
    ).publicKey;

    // 从IDL文件获取接口定义
    const idl = require("../target/idl/wusd_token.json");
    program = new anchor.Program(idl, programId, provider);
  }

  // 计算PDA地址
  const [authorityState, authorityBump] =
    await anchor.web3.PublicKey.findProgramAddress(
      [Buffer.from("authority"), tokenMint.toBuffer()],
      program.programId
    );

  const [mintState] = await anchor.web3.PublicKey.findProgramAddress(
    [Buffer.from("mint_state"), tokenMint.toBuffer()],
    program.programId
  );

  const [pauseState] = await anchor.web3.PublicKey.findProgramAddress(
    [Buffer.from("pause_state"), tokenMint.toBuffer()],
    program.programId
  );

  const [freezeState] = await anchor.web3.PublicKey.findProgramAddress(
    [
      Buffer.from("freeze"),
      recipientTokenAccount.toBuffer(),
      tokenMint.toBuffer(),
    ],
    program.programId
  ); 

  console.log("Authority state:", authorityState.toBase58());
  console.log("Mint state:", mintState.toBase58());
  console.log("Pause state:", pauseState.toBase58());
  console.log("Freeze state:", freezeState.toBase58());

  // 设置代币小数位数
  const decimals = 9; // 根据test.ts中使用的小数位数修改为9

  // 显示铸币金额
  console.log(
    "Attempting to mint",
    amount.div(new BN(10 ** decimals)).toString(),
    "WUSD"
  );

  try {
    // 创建一个临时账户用于部分签名
    const mintAccount = Keypair.generate();
    fs.writeFileSync(
      "./mintAccount.json",
      JSON.stringify(Array.from(mintAccount.secretKey))
    );
    
    // 使用参考test.ts中的方式创建mint指令
    const mintIx = await program.methods
      .mint(amount, authorityBump)
      .accounts({
        authority: mintAccount.publicKey,
        tokenMint: tokenMint,
        tokenAccount: recipientTokenAccount,
        authorityState: authorityState,
        mintState: mintState,
        pauseState: pauseState,
        freezeState: freezeState,
        tokenProgram: TOKEN_2022_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([mintAccount])
      .instruction();

    // 创建交易
    const transaction = new Transaction();
    transaction.add(mintIx);
    transaction.feePayer = minter;
    
    // 重要：先获取recentBlockhash，再部分签名
    transaction.recentBlockhash = (
      await connection.getLatestBlockhash()
    ).blockhash;
    
    // 部分签名
    transaction.partialSign(mintAccount);

    // 发送并确认交易
    console.log("Sending mint transaction...");
    const signature = await sendAndConfirmTransaction(
      connection,
      transaction,
      [] // Fireblocks已经处理minter的签名，不需要额外的签名者
    );

    console.log("Tokens minted successfully!");
    console.log(
      `Transaction: https://explorer.solana.com/tx/${signature}?cluster=devnet`
    );
    return signature;
  } catch (error) {
    console.error("Error minting tokens:", error);
    throw error;
  }
};

mintToken();
