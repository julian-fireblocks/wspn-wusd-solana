import {
  Keypair,
  PublicKey,
  Transaction,
  clusterApiUrl,
  sendAndConfirmTransaction,
} from "@solana/web3.js";
import {
  FireblocksConnectionAdapter,
  FireblocksConnectionAdapterConfig,
  FeeLevel,
} from "./fireblocks/index";
import { TOKEN_2022_PROGRAM_ID } from "@solana/spl-token";
import * as anchor from "@coral-xyz/anchor";
import * as fs from "fs";

require("dotenv").config();

const InitContract = async () => {
  let program: any;
  const decimals = 9;
  let programId: anchor.web3.PublicKey;
  let authorityState: anchor.web3.PublicKey;
  let mintState: anchor.web3.PublicKey;
  let pauseState: anchor.web3.PublicKey;
  console.log("Starting contract initialization...");

  // 配置Fireblocks连接
  const fireblocksConnectionConfig: FireblocksConnectionAdapterConfig = {
    apiKey: process.env.FIREBLOCKS_API_KEY || "",
    apiSecretPath: process.env.FIREBLOCKS_SECRET_KEY_PATH || "",
    vaultAccountId: process.env.ADMIN_VAULT_ACCOUNT_ID || "",
    feeLevel: FeeLevel.HIGH, // 使用高费用级别进行部署交易
    silent: false,
    devnet: true, // 设置为true表示devnet，false表示mainnet
  };

  // 创建到Solana devnet的连接
  const connection = await FireblocksConnectionAdapter.create(
    clusterApiUrl("devnet"),
    fireblocksConnectionConfig
  );

  // 获取Fireblocks钱包的公钥作为管理员
  const admin = new PublicKey(connection.getAccount());
  console.log("Admin account:", admin.toBase58());

  // 创建铸币者和暂停者的密钥对
  const minterPublicKey = new PublicKey(process.env.WUSD_MINTER_ADDRESS || "");
  const pauserPublicKey = new PublicKey(process.env.WUSD_PAUSER_ADDRESS || "");
  console.log("Minter account:", minterPublicKey.toBase58());
  console.log("Pauser account:", pauserPublicKey.toBase58());

  // 创建代币铸造密钥对
  // 从环境变量或文件中加载tokenMint的密钥对
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
  try {
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

    // 预先计算所有PDA
    [authorityState] = await anchor.web3.PublicKey.findProgramAddress(
      [Buffer.from("authority"), tokenMint.toBuffer()],
      program.programId
    );

    [mintState] = await anchor.web3.PublicKey.findProgramAddress(
      [Buffer.from("mint_state"), tokenMint.toBuffer()],
      program.programId
    );

    [pauseState] = await anchor.web3.PublicKey.findProgramAddress(
      [Buffer.from("pause_state"), tokenMint.toBuffer()],
      program.programId
    );

    console.log("Authority state:", authorityState.toBase58());
    console.log("Mint state:", mintState.toBase58());
    console.log("Pause state:", pauseState.toBase58());
    const targetAccount = Keypair.generate(); 
    // 创建交易
    const initTx = new Transaction().add(
      await program.methods
        .initialize(decimals)
        .accounts({
          authority: targetAccount,
          minter: minterPublicKey,
          pauser: pauserPublicKey,
          tokenMint: tokenMint,
          authorityState: authorityState,
          mintState: mintState,
          pauseState: pauseState,
          tokenProgram: TOKEN_2022_PROGRAM_ID,
          systemProgram: anchor.web3.SystemProgram.programId,
          rent: anchor.web3.SYSVAR_RENT_PUBKEY,
        })
        .signers([targetAccount])
        .instruction()
    );
    initTx.recentBlockhash = (await connection.getLatestBlockhash()).blockhash;
    initTx.feePayer = admin;
    initTx.partialSign(targetAccount);
    const hash = await sendAndConfirmTransaction(connection, initTx, []);
    console.log(
      `Init contract: https://explorer.solana.com/tx/${hash}?cluster=devnet`
    );
  } catch (error) {
    console.error("Error initializing contract:", error);
    throw error;
  }
};

InitContract();
