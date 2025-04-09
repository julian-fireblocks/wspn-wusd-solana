import * as nacl from "tweetnacl";
import {
  LAMPORTS_PER_SOL,
  SystemProgram,
  PublicKey,
  Keypair,
} from "@solana/web3.js";
import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import {
  TOKEN_2022_PROGRAM_ID,
  createInitializeMint2Instruction,
  createAssociatedTokenAccountInstruction,
  getAssociatedTokenAddressSync, 
} from "@solana/spl-token";
import { WusdToken } from "../target/types/wusd_token";
import { assert } from "chai";

describe("WUSD Token Test", () => {
  // 1. 首先定义所有变量
  let provider: anchor.AnchorProvider;

  before(async () => {
    try {
      provider = anchor.AnchorProvider.env();
      if (!provider) {
        throw new Error("AnchorProvider not properly initialized");
      }
      anchor.setProvider(provider);
      console.log("\n=== Provider Initialization ===\nProvider wallet:", provider.wallet.publicKey.toString());
    } catch (error) {
      console.error("\nError during provider initialization:", error);
      throw error;
    }
  });


  // 确保程序ID已正确初始化
  const programId = new PublicKey(
    "4xPf5n8CbNUm8AT5DVgdWaPT3nVTPKU9oGjreiGBK3fB"
  );

  // 确保程序正确加载
  let program: Program<WusdToken>;
  program = anchor.workspace.WusdToken as Program<WusdToken>;
  if (!program || !program.programId) {
    console.log(
      "Program not found in workspace, will load from IDL during initialization"
    );
  }

  // 定义关键账户
  let mintKeypair: Keypair;
  let recipientKeypair: Keypair;
  let minterKeypair: Keypair;
  let pauserKeypair: Keypair;

  // 定义PDA账户
  let authorityPda: PublicKey;
  let mintStatePda: PublicKey;
  let pauseStatePda: PublicKey;
  let authorityBump: number;

  // 定义代币账户
  let recipientTokenAccount: PublicKey;

  const sleep = (ms: number) =>
    new Promise((resolve) => setTimeout(resolve, ms));

  before(async () => {
    try {
      console.log("\n=== Starting Test Setup ===");
      console.log("Starting initialization with simplified approach...");

      // 1. 生成密钥对
      mintKeypair = anchor.web3.Keypair.generate();
      recipientKeypair = anchor.web3.Keypair.generate();
      minterKeypair = anchor.web3.Keypair.generate();
      pauserKeypair = anchor.web3.Keypair.generate();

      console.log("Generated keypairs:");
      console.log("Mint keypair:", mintKeypair.publicKey.toString());
      console.log("Recipient keypair:", recipientKeypair.publicKey.toString());

      // 检查连接是否正确指向devnet
      const endpoint = provider.connection.rpcEndpoint;
      console.log("Connected to:", endpoint);
      if (!endpoint.includes("devnet")) {
        console.warn(
          "Warning: Not connected to devnet! Current endpoint:",
          endpoint
        );
      }

      // 2. 跳过空投步骤，在devnet上使用已有的SOL
      console.log(
        "Skipping airdrop on devnet - please ensure your wallet already has SOL"
      );
      console.log("Wallet address:", provider.wallet.publicKey.toString());
      console.log("Recipient address:", recipientKeypair.publicKey.toString());

      // 3. 计算 PDA 地址
      console.log("Calculating PDA addresses...");
      // 使用显式定义的programId，因为program.programId可能未定义
      [authorityPda, authorityBump] = PublicKey.findProgramAddressSync(
        [Buffer.from("authority"), mintKeypair.publicKey.toBuffer()],
        programId
      );

      [mintStatePda] = PublicKey.findProgramAddressSync(
        [Buffer.from("mint_state"), mintKeypair.publicKey.toBuffer()],
        programId
      );

      [pauseStatePda] = PublicKey.findProgramAddressSync(
        [Buffer.from("pause_state"), mintKeypair.publicKey.toBuffer()],
        programId
      );

      // 4. 初始化合约状态
      console.log("Initializing contract state...");
      try {
        console.log("Debug: Account addresses being used:");
        console.log("Authority:", provider.wallet.publicKey.toString());
        console.log("Mint:", mintKeypair.publicKey.toString());
        console.log("Authority PDA:", authorityPda.toString());
        console.log("Mint State PDA:", mintStatePda.toString());
        console.log("Pause State PDA:", pauseStatePda.toString());

        // 重新加载程序实例，确保程序正确初始化
        if (!program || !program.programId) {
          console.log("Program not found in workspace, loading from IDL...");
          try {
            // 尝试从本地加载IDL
            const idl = JSON.parse(
              require("fs").readFileSync("./target/idl/wusd_token.json", "utf8")
            );
            program = new anchor.Program(
              idl,
              programId,
              provider
            ) as Program<WusdToken>;
          } catch (idlError) {
            console.error(
              "Error loading local IDL, attempting to fetch from network:",
              idlError
            );
            // 如果本地加载失败，尝试从网络获取
            const idl = await anchor.Program.fetchIdl(programId, provider);
            if (!idl) {
              throw new Error(
                "IDL could not be fetched from network and local file not found"
              );
            }
            program = new anchor.Program(
              idl,
              programId,
              provider
            ) as Program<WusdToken>;
          }
        }

        // 获取租金豁免金额
        const mintSize = 82; // Token2022 Mint账户的标准大小
        const rentExemptAmount =
          await provider.connection.getMinimumBalanceForRentExemption(mintSize);

        // 检查mint账户是否已存在
        const existingMintAccount = await provider.connection.getAccountInfo(mintKeypair.publicKey);
        
        // 只有当mint账户不存在时才创建
        if (!existingMintAccount) {
          try {
            // 创建铸币账户指令
            const createAccountIx = SystemProgram.createAccount({
              fromPubkey: provider.wallet.publicKey,
              newAccountPubkey: mintKeypair.publicKey,
              space: mintSize,
              lamports: rentExemptAmount,
              programId: TOKEN_2022_PROGRAM_ID,
            });

            // 添加Token铸币初始化指令
            const createMintIx = createInitializeMint2Instruction(
              mintKeypair.publicKey,
              6, // 6位小数
              provider.wallet.publicKey, // 先使用钱包作为铸币权限，后续再转移给PDA
              null,
              TOKEN_2022_PROGRAM_ID
            );

            // 创建交易并添加指令
            const tx = new anchor.web3.Transaction()
              .add(createAccountIx)
              .add(createMintIx);

            // 获取最新区块哈希
            const { blockhash } = await provider.connection.getLatestBlockhash();
            tx.recentBlockhash = blockhash;
            tx.feePayer = provider.wallet.publicKey;

            // 签名交易
            tx.partialSign(mintKeypair);

            console.log("Sending transaction to create mint account...");
            await provider.sendAndConfirm(tx, [mintKeypair]);
            console.log("Mint account created successfully");
          } catch (error) {
            console.error("Error creating mint account:", error);
            throw new Error("Failed to create mint account");
          }
        } else {
          console.log("Mint account already exists, skipping creation");
        }

        // 使用已生成的minter和pauser密钥对
        console.log("Using existing keypairs:");
        console.log("Minter:", minterKeypair.publicKey.toString());
        console.log("Pauser:", pauserKeypair.publicKey.toString());

        // 为minter和pauser空投SOL
        try {
          const airdropTx1 = await provider.connection.requestAirdrop(
            minterKeypair.publicKey,
            1 * LAMPORTS_PER_SOL
          );
          await provider.connection.confirmTransaction(airdropTx1);
          
          const airdropTx2 = await provider.connection.requestAirdrop(
            pauserKeypair.publicKey,
            1 * LAMPORTS_PER_SOL
          );
          await provider.connection.confirmTransaction(airdropTx2);
        } catch (error) {
          console.error("Error airdropping SOL to minter/pauser:", error);
          throw new Error("Failed to airdrop SOL to minter/pauser");
        }

        // 初始化合约状态
        try {

          console.log("Debug: Account addresses being used:");
          console.log("Authority:", provider.wallet.publicKey.toString());
          console.log("Mint:", mintKeypair.publicKey.toString());
          console.log("Authority PDA:", authorityPda.toString());
          console.log("Mint State PDA:", mintStatePda.toString());
          console.log("Pause State PDA:", pauseStatePda.toString());
          console.log("Minter:", minterKeypair.publicKey.toString());
          console.log("Pauser:", pauserKeypair.publicKey.toString());

          // 检查所有账户状态
          console.log("Checking all account states...");
          const accountStates = await Promise.all([
            provider.connection.getAccountInfo(mintKeypair.publicKey),
            provider.connection.getAccountInfo(mintStatePda),
            provider.connection.getAccountInfo(authorityPda),
            provider.connection.getAccountInfo(pauseStatePda)
          ]);

          const [checkMintAccount, checkMintState, checkAuthorityState, checkPauseState] = accountStates;

          // 如果所有账户都已初始化，则跳过初始化步骤
          if (
            checkMintAccount &&
            checkMintAccount.owner.equals(TOKEN_2022_PROGRAM_ID) &&
            checkMintState &&
            checkAuthorityState &&
            checkPauseState
          ) {
            console.log("All accounts already exist and initialized, skipping initialization");
            return;
          }

          // 只有当mint账户不存在时才创建
          if (!checkMintAccount) {
            try {
              // 创建铸币账户指令
              const createAccountIx = SystemProgram.createAccount({
                fromPubkey: provider.wallet.publicKey,
                newAccountPubkey: mintKeypair.publicKey,
                space: mintSize,
                lamports: rentExemptAmount,
                programId: TOKEN_2022_PROGRAM_ID,
              });

              // 添加Token铸币初始化指令
              const createMintIx = createInitializeMint2Instruction(
                mintKeypair.publicKey,
                6, // 6位小数
                provider.wallet.publicKey, // 先使用钱包作为铸币权限，后续再转移给PDA
                null,
                TOKEN_2022_PROGRAM_ID
              );

              // 创建交易并添加指令
              const tx = new anchor.web3.Transaction()
                .add(createAccountIx)
                .add(createMintIx);

              // 获取最新区块哈希
              const { blockhash } = await provider.connection.getLatestBlockhash();
              tx.recentBlockhash = blockhash;
              tx.feePayer = provider.wallet.publicKey;

              // 签名交易
              tx.partialSign(mintKeypair);

              console.log("Sending transaction to create mint account...");
              await provider.sendAndConfirm(tx, [mintKeypair]);
              console.log("Mint account created successfully");
              await sleep(1000); // 等待账户更新
            } catch (error) {
              console.error("Error creating mint account:", error);
              throw new Error("Failed to create mint account");
            }
          } else {
            console.log("Mint account already exists, skipping creation");
          }

          // 为minter和pauser空投SOL
          try {
            const airdropTx1 = await provider.connection.requestAirdrop(
              minterKeypair.publicKey,
              1 * LAMPORTS_PER_SOL
            );
            await provider.connection.confirmTransaction(airdropTx1);
            console.log("Airdropped SOL to minter:", minterKeypair.publicKey.toString());
            
            const airdropTx2 = await provider.connection.requestAirdrop(
              pauserKeypair.publicKey,
              1 * LAMPORTS_PER_SOL
            );
            await provider.connection.confirmTransaction(airdropTx2);
            console.log("Airdropped SOL to pauser:", pauserKeypair.publicKey.toString());
          } catch (error) {
            console.error("Error airdropping SOL to minter/pauser:", error);
            throw new Error("Failed to airdrop SOL to minter/pauser");
          }

          // 检查PDA账户是否已存在
          console.log("Checking PDA accounts status...");
          const mintStateInfo = await provider.connection.getAccountInfo(mintStatePda);
          const authorityStateInfo = await provider.connection.getAccountInfo(authorityPda);
          const pauseStateInfo = await provider.connection.getAccountInfo(pauseStatePda);

          console.log("PDA accounts status:");
          console.log("- Mint State:", mintStateInfo ? "exists" : "not exists");
          console.log("- Authority State:", authorityStateInfo ? "exists" : "not exists");
          console.log("- Pause State:", pauseStateInfo ? "exists" : "not exists");

          // 如果任何PDA账户不存在，则执行初始化
          if (!mintStateInfo || !authorityStateInfo || !pauseStateInfo) {
            console.log("Initializing PDA accounts...");
            try {
              const tx = await program.methods
                .initialize(6) // 传入6位小数
                .accounts({
                  authority: provider.wallet.publicKey,
                  minter: minterKeypair.publicKey,
                  pauser: pauserKeypair.publicKey,
                  authorityState: authorityPda,
                  tokenMint: mintKeypair.publicKey,
                  mintState: mintStatePda,
                  pauseState: pauseStatePda,
                  systemProgram: SystemProgram.programId,
                  tokenProgram: TOKEN_2022_PROGRAM_ID,
                })
                .signers([mintKeypair]) // 添加mintKeypair作为签名者
                .rpc();

              console.log("PDA accounts initialization transaction:", tx);
              await provider.connection.confirmTransaction(tx);
              console.log("PDA accounts initialized successfully");
              await sleep(1000); // 等待状态更新

              // 验证PDA账户是否已正确初始化
              const verifyMintState = await provider.connection.getAccountInfo(mintStatePda);
              const verifyAuthorityState = await provider.connection.getAccountInfo(authorityPda);
              const verifyPauseState = await provider.connection.getAccountInfo(pauseStatePda);

              console.log("\nVerification after initialization:");
              console.log("- Mint State:", verifyMintState ? "initialized" : "failed");
              console.log("- Authority State:", verifyAuthorityState ? "initialized" : "failed");
              console.log("- Pause State:", verifyPauseState ? "initialized" : "failed");

              if (!verifyMintState || !verifyAuthorityState || !verifyPauseState) {
                throw new Error("PDA accounts verification failed after initialization");
              }
            } catch (initError) {
              console.error("\nError during PDA accounts initialization:", initError);
              console.error("Error details:", JSON.stringify(initError, null, 2));
              throw initError;
            }
          } else {
            console.log("PDA accounts already initialized, skipping initialization");
          }

          // 检查账户是否已经存在
          const existingMintAccount = await provider.connection.getAccountInfo(
            mintKeypair.publicKey
          );
          const existingMintState = await provider.connection.getAccountInfo(
            mintStatePda
          );
          const existingAuthorityState = await provider.connection.getAccountInfo(
            authorityPda
          );
          const existingPauseState = await provider.connection.getAccountInfo(
            pauseStatePda
          );

          // 如果所有账户都已初始化，则跳过初始化步骤
          if (
            existingMintAccount &&
            existingMintAccount.owner.equals(TOKEN_2022_PROGRAM_ID) &&
            existingMintState &&
            existingAuthorityState &&
            existingPauseState
          ) {
            console.log(
              "All accounts already exist and initialized, skipping initialization"
            );
            return;
          }

          // 执行初始化流程
          console.log("Initializing contract state...");
          try {
            await program.methods
              .initialize(6) // 传入6位小数
              .accounts({
                authority: provider.wallet.publicKey,
                minter: minterKeypair.publicKey,
                pauser: pauserKeypair.publicKey,
                authorityState: authorityPda,
                tokenMint: mintKeypair.publicKey,
                mintState: mintStatePda,
                pauseState: pauseStatePda,
                systemProgram: SystemProgram.programId,
                tokenProgram: TOKEN_2022_PROGRAM_ID,
              })
              .signers([mintKeypair])
              .rpc();

            console.log("Contract state initialized successfully");
            await sleep(1000); // 等待状态更新
          } catch (error) {
            console.error("Error in initialization:", error);
            throw error;
          }
        } catch (error) {
          console.error("Error initializing contract state:", error);
          throw error;
        }

        // 创建接收者的代币账户
        console.log("Creating recipient token account...");
      } catch (error) {
        console.error("Setup failed:", error);
        throw error;
      }
    } catch (error) {
      console.error("Before hook failed:", error);
      throw error;
    }
  });

  it("Create Recipient Token Account", async () => {
    try {
      // 获取关联代币账户地址
      recipientTokenAccount = getAssociatedTokenAddressSync(
        mintKeypair.publicKey,
        recipientKeypair.publicKey,
        false,
        TOKEN_2022_PROGRAM_ID
      );

      // 创建关联代币账户指令
      const createTokenAccountIx = createAssociatedTokenAccountInstruction(
        provider.wallet.publicKey,
        recipientTokenAccount,
        recipientKeypair.publicKey,
        mintKeypair.publicKey,
        TOKEN_2022_PROGRAM_ID
      );

      // 创建并发送交易
      const tx = new anchor.web3.Transaction().add(createTokenAccountIx);
      tx.feePayer = provider.wallet.publicKey;
      
      const { blockhash } = await provider.connection.getLatestBlockhash();
      tx.recentBlockhash = blockhash;
      
      const signature = await provider.sendAndConfirm(tx);
      console.log(
        "Recipient token account created:",
        recipientTokenAccount.toString()
      );
    } catch (error) {
      console.error("Token account creation failed:", error);
      throw error;
    }
  });

  it("Set Minter Role and Mint WUSD tokens", async () => {
    try {
      // 首先检查authority_state账户是否已初始化
      const authorityStateInfo = await provider.connection.getAccountInfo(authorityPda);
      if (!authorityStateInfo) {
        console.log("Authority state account not initialized, skipping test");
        return;
      }

      console.log("Debug mint operation:");
      console.log("Current Authority:", provider.wallet.publicKey.toString());

      // 使用已创建的Minter账户
      console.log("Using existing Minter:", minterKeypair.publicKey.toString());

      // 转账SOL给minter账户以支付交易费用
      const transferToMinterTx = new anchor.web3.Transaction().add(
        SystemProgram.transfer({
          fromPubkey: provider.wallet.publicKey,
          toPubkey: minterKeypair.publicKey,
          lamports: LAMPORTS_PER_SOL * 0.1, // 转0.1 SOL
        })
      );
      await provider.sendAndConfirm(transferToMinterTx);
      console.log("SOL transferred to minter account");

      // 1. 首先由admin账户调用set_role来配置Minter角色
      console.log("Setting minter role...");
      const setRoleTx = await program.methods
        .setRole({ minter: {} }, minterKeypair.publicKey)
        .accounts({
          admin: provider.wallet.publicKey,
          authorityState: authorityPda,
          tokenMint: mintKeypair.publicKey,
        })
        .signers([provider.wallet.payer])
        .rpc();

      await provider.connection.confirmTransaction(setRoleTx);
      console.log("Successfully set minter role to:", minterKeypair.publicKey.toString());

      // 2. 然后由新设置的Minter角色账户执行铸币操作
      console.log("Executing mint operation with new minter...");
      const mintTx = await program.methods
        .mint(new anchor.BN(10000000000), authorityBump)
        .accounts({
          authority: minterKeypair.publicKey,
          tokenMint: mintKeypair.publicKey,
          tokenAccount: recipientTokenAccount,
          tokenProgram: TOKEN_2022_PROGRAM_ID,
          authorityState: authorityPda,
          mintState: mintStatePda,
          pauseState: pauseStatePda,
          systemProgram: SystemProgram.programId,
        })
        .signers([minterKeypair])
        .rpc();

      await provider.connection.confirmTransaction(mintTx);
      console.log("Successfully minted WUSD tokens");

      // 验证铸币结果
      const tokenAccount = await provider.connection.getTokenAccountBalance(
        recipientTokenAccount
      );
      console.log("Token balance:", tokenAccount.value.uiAmount);
    } catch (error) {
      console.error("Minting failed:", error);
      throw error;
    }
  });

  it("Transfer WUSD tokens", async () => {
    try {
      // 跳过为 recipientKeypair 请求空投，在devnet上使用已有的SOL
      console.log(
        "Skipping airdrop to recipient on devnet - please ensure your wallet already has SOL"
      );
      console.log("Recipient address:", recipientKeypair.publicKey.toString());
      await sleep(1000); // 等待一下以确保连接稳定
      // 创建新的接收账户
      const newRecipient = Keypair.generate();
      const newRecipientTokenAccount = getAssociatedTokenAddressSync(
        mintKeypair.publicKey,
        newRecipient.publicKey,
        false,
        TOKEN_2022_PROGRAM_ID
      );

      // 创建接收账户的代币账户
      const createTokenAccountIx = createAssociatedTokenAccountInstruction(
        provider.wallet.publicKey,
        newRecipientTokenAccount,
        newRecipient.publicKey,
        mintKeypair.publicKey,
        TOKEN_2022_PROGRAM_ID
      );

      const tx = new anchor.web3.Transaction().add(createTokenAccountIx);
      const signature = await provider.sendAndConfirm(tx);
      await provider.connection.confirmTransaction(signature, "confirmed");

      // 获取转账前的余额
      const balanceBefore = await provider.connection.getTokenAccountBalance(
        recipientTokenAccount
      );
      console.log(
        "Sender balance before transfer:",
        balanceBefore.value.uiAmount
      );

      // 正确派生 freeze state PDAs
      const [fromFreezeState] = PublicKey.findProgramAddressSync(
        [Buffer.from("freeze"), recipientTokenAccount.toBuffer()],
        program.programId
      );

      const [toFreezeState] = PublicKey.findProgramAddressSync(
        [Buffer.from("freeze"), newRecipientTokenAccount.toBuffer()],
        program.programId
      );

      // 检查 from_freeze_state 是否已存在
      let fromFreezeStateExists = false;
      try {
        await program.account.freezeState.fetch(fromFreezeState);
        fromFreezeStateExists = true;
        console.log("From freeze state already exists");
      } catch (error) {
        // 账户不存在，需要初始化
        fromFreezeStateExists = false;
      }

      // 如果不存在，则初始化 from_freeze_state
      if (!fromFreezeStateExists) {
        const initFromFreezeStateTx = await program.methods
          .initializeFreezeState()
          .accounts({
            authority: provider.wallet.publicKey,
            freezeState: fromFreezeState,
            tokenAccount: recipientTokenAccount,
            payer: provider.wallet.publicKey,
            systemProgram: SystemProgram.programId,
            tokenProgram: TOKEN_2022_PROGRAM_ID,
          })
          .rpc();

        await provider.connection.confirmTransaction(initFromFreezeStateTx);
        console.log("Initialized from freeze state");
      }

      // 检查 to_freeze_state 是否已存在
      let toFreezeStateExists = false;
      try {
        await program.account.freezeState.fetch(toFreezeState);
        toFreezeStateExists = true;
        console.log("To freeze state already exists");
      } catch (error) {
        // 账户不存在，需要初始化
        toFreezeStateExists = false;
      }

      // 如果不存在，则初始化 to_freeze_state
      if (!toFreezeStateExists) {
        const initToFreezeStateTx = await program.methods
          .initializeFreezeState()
          .accounts({
            authority: provider.wallet.publicKey,
            freezeState: toFreezeState,
            tokenAccount: newRecipientTokenAccount,
            payer: provider.wallet.publicKey,
            systemProgram: SystemProgram.programId,
            tokenProgram: TOKEN_2022_PROGRAM_ID,
          })
          .rpc();

        await provider.connection.confirmTransaction(initToFreezeStateTx);
        console.log("Initialized to freeze state");
      }

      // 执行转账操作
      const transferAmount = new anchor.BN(5000000); // 5 WUSD

      const transferTx = await program.methods
        .transfer(transferAmount)
        .accounts({
          from: recipientKeypair.publicKey,
          to: newRecipient.publicKey,
          fromToken: recipientTokenAccount,
          toToken: newRecipientTokenAccount,
          tokenProgram: TOKEN_2022_PROGRAM_ID,
          tokenMint: mintKeypair.publicKey,  // 添加缺失的tokenMint参数
          pauseState: pauseStatePda,
          fromFreezeState: fromFreezeState,
          toFreezeState: toFreezeState,
        })
        .signers([recipientKeypair])
        .rpc();

      await provider.connection.confirmTransaction(transferTx, "confirmed");

      // 验证转账结果
      const senderBalanceAfter =
        await provider.connection.getTokenAccountBalance(recipientTokenAccount);
      const receiverBalance = await provider.connection.getTokenAccountBalance(
        newRecipientTokenAccount
      );

      console.log(
        "Sender balance after transfer:",
        senderBalanceAfter.value.uiAmount
      );
      console.log("Receiver balance:", receiverBalance.value.uiAmount);

      // 验证余额变化
      const expectedSenderBalance =
        balanceBefore.value.uiAmount - transferAmount.toNumber() / 1000000;
      assert.approximately(
        senderBalanceAfter.value.uiAmount,
        expectedSenderBalance,
        0.000001,
        "Transfer amount not correctly deducted from sender"
      );

      assert.approximately(
        receiverBalance.value.uiAmount,
        transferAmount.toNumber() / 1000000,
        0.000001,
        "Transfer amount not correctly added to receiver"
      );

      console.log("Transfer operation successful");
    } catch (error) {
      console.error("Transfer operation failed:", error);
      throw error;
    }
  });

  it("Test transfer_from functionality", async () => {
    try {
      // 首先检查authority_state账户是否已初始化
      const authorityStateInfo = await provider.connection.getAccountInfo(authorityPda);
      if (!authorityStateInfo) {
        console.log("Authority state account not initialized, skipping test");
        return;
      }

      console.log("Recipient address:", recipientKeypair.publicKey.toString());

      // 创建一个新的spender账户
      const spenderKeypair = anchor.web3.Keypair.generate();
      console.log("Spender address:", spenderKeypair.publicKey.toString());

      // 转账SOL给spender和recipient账户以支付交易费用
      console.log("Transferring SOL to spender and recipient for account creation fees");
      const transferTx = new anchor.web3.Transaction().add(
        SystemProgram.transfer({
          fromPubkey: provider.wallet.publicKey,
          toPubkey: spenderKeypair.publicKey,
          lamports: LAMPORTS_PER_SOL * 0.1, // 转0.1 SOL
        }),
        SystemProgram.transfer({
          fromPubkey: provider.wallet.publicKey,
          toPubkey: recipientKeypair.publicKey,
          lamports: LAMPORTS_PER_SOL * 0.1, // 转0.1 SOL
        })
      );
      await provider.sendAndConfirm(transferTx);
      console.log("SOL transferred to spender and recipient accounts");

      // 创建spender的代币账户
      const spenderTokenAccount = getAssociatedTokenAddressSync(
        mintKeypair.publicKey,
        spenderKeypair.publicKey,
        false,
        TOKEN_2022_PROGRAM_ID
      );

      const createSpenderTokenAccountIx = createAssociatedTokenAccountInstruction(
        provider.wallet.publicKey,
        spenderTokenAccount,
        spenderKeypair.publicKey,
        mintKeypair.publicKey,
        TOKEN_2022_PROGRAM_ID
      );

      const createSpenderTokenTx = new anchor.web3.Transaction().add(
        createSpenderTokenAccountIx
      );
      await provider.sendAndConfirm(createSpenderTokenTx);
      console.log("Created spender token account");

      // 计算allowance和permit PDA
      const [allowanceStatePda] = PublicKey.findProgramAddressSync(
        [
          Buffer.from("allowance"),
          recipientKeypair.publicKey.toBuffer(),
          spenderKeypair.publicKey.toBuffer(),
        ],
        program.programId
      );

      const [permitPda] = PublicKey.findProgramAddressSync(
        [
          Buffer.from("permit"),
          recipientKeypair.publicKey.toBuffer(),
          spenderKeypair.publicKey.toBuffer(),
        ],
        program.programId
      );

      console.log("Debug PDA addresses:", {
        allowanceStatePda: allowanceStatePda.toString(),
        permitPda: permitPda.toString(),
        owner: recipientKeypair.publicKey.toString(),
        spender: spenderKeypair.publicKey.toString(),
      });

      // 执行transfer_from操作
      const transferAmount = new anchor.BN(1000000); // 1 WUSD

      // 首先需要approve操作
      const approveTx = await program.methods
        .approve(transferAmount)
        .accounts({
          owner: recipientKeypair.publicKey,
          spender: spenderKeypair.publicKey,
          ownerTokenAccount: recipientTokenAccount,
          allowanceState: allowanceStatePda,
          tokenMint: mintKeypair.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .signers([recipientKeypair])
        .rpc();

      await provider.connection.confirmTransaction(approveTx);

      // 然后执行transfer_from操作
      const transferFromTx = await program.methods
        .transferFrom(transferAmount)
        .accounts({
          spender: spenderKeypair.publicKey,
          owner: recipientKeypair.publicKey,
          ownerTokenAccount: recipientTokenAccount,
          recipientTokenAccount: spenderTokenAccount,
          allowanceState: allowanceStatePda,
          tokenProgram: TOKEN_2022_PROGRAM_ID,
          tokenMint: mintKeypair.publicKey,  // 添加缺失的tokenMint参数
          pauseState: pauseStatePda,
        })
        .signers([spenderKeypair])
        .rpc();

      await provider.connection.confirmTransaction(transferFromTx);
      console.log("Successfully executed transfer_from");

      // 验证转账结果
      const ownerBalance = await provider.connection.getTokenAccountBalance(
        recipientTokenAccount
      );
      const spenderBalance = await provider.connection.getTokenAccountBalance(
        spenderTokenAccount
      );

      console.log("Owner balance after transfer:", ownerBalance.value.uiAmount);
      console.log("Spender balance after transfer:", spenderBalance.value.uiAmount);
    } catch (error) {
      console.error("Transfer_from failed:", error);
      throw error;
    }
  });

  it("Burn WUSD tokens", async () => {
    try {
      // 首先检查authority_state账户是否已初始化
      const authorityStateInfo = await provider.connection.getAccountInfo(authorityPda);
      if (!authorityStateInfo) {
        console.log("Authority state account not initialized, skipping test");
        return;
      }

      console.log("Starting burn test...");
      // 获取销毁前的余额
      const balanceBefore = await provider.connection.getTokenAccountBalance(
        recipientTokenAccount
      );
      console.log("Balance before burn:", balanceBefore.value.uiAmount);

      // 创建一个新的Burner角色账户
      const burnerKeypair = anchor.web3.Keypair.generate();

      // 转账SOL给burner账户以支付交易费用
      const transferToBurnerTx = new anchor.web3.Transaction().add(
        SystemProgram.transfer({
          fromPubkey: provider.wallet.publicKey,
          toPubkey: burnerKeypair.publicKey,
          lamports: LAMPORTS_PER_SOL * 0.1, // 转0.1 SOL
        })
      );
      await provider.sendAndConfirm(transferToBurnerTx);

      // 设置Burner角色
      const setRoleTx = await program.methods
        .setRole({ burner: {} }, burnerKeypair.publicKey)
        .accounts({
          admin: provider.wallet.publicKey,
          authorityState: authorityPda,
          tokenMint: mintKeypair.publicKey,
        })
        .signers([provider.wallet.payer])
        .rpc();

      await provider.connection.confirmTransaction(setRoleTx);

      // 执行销毁操作
      const burnAmount = new anchor.BN(1000000); // 1 WUSD
      const burnTx = await program.methods
        .burn(burnAmount)
        .accounts({
          authority: burnerKeypair.publicKey,
          tokenMint: mintKeypair.publicKey,
          tokenAccount: recipientTokenAccount,
          tokenProgram: TOKEN_2022_PROGRAM_ID,
          authorityState: authorityPda,
          mintState: mintStatePda,
          pauseState: pauseStatePda,
          systemProgram: SystemProgram.programId,
        })
        .signers([burnerKeypair])
        .rpc();

      await provider.connection.confirmTransaction(burnTx);
      console.log("Successfully burned WUSD tokens");

      // 验证销毁结果
      const balanceAfter = await provider.connection.getTokenAccountBalance(
        recipientTokenAccount
      );
      console.log("Balance after burn:", balanceAfter.value.uiAmount);
    } catch (error) {
      console.error("Burn operation failed:", error);
      throw error;
    }
  });


});
