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
  createApproveInstruction,
} from "@solana/spl-token";
import { WusdToken } from "../target/types/wusd_token";
import { assert } from "chai";

describe("WUSD Token Test", () => {
  // 1. 首先定义所有变量
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  // 确保程序ID已正确初始化
  const programId = new PublicKey(
    "8nBbkdsTkqbrnrbVTUxyciQNvT6Q5B3pZkPQmP3nnuwU"
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
      console.log("Starting initialization with simplified approach...");

      // 1. 生成密钥对
      mintKeypair = anchor.web3.Keypair.generate();
      recipientKeypair = anchor.web3.Keypair.generate();

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

        // 使用简单直接的方法创建Token 2022账户
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

        // 初始化合约状态
        try {
          // 初始化合约状态
          console.log("Initializing contract state with program...");

          // 检查账户是否已经存在
          const mintAccountInfo = await provider.connection.getAccountInfo(
            mintKeypair.publicKey
          );
          const mintStateInfo = await provider.connection.getAccountInfo(
            mintStatePda
          );
          const authorityStateInfo = await provider.connection.getAccountInfo(
            authorityPda
          );

          // 如果账户已经存在并且是Token 2022账户，则不需要再次创建
          // 但我们仍然需要确保authorityState和mintState已初始化
          if (
            mintAccountInfo &&
            mintAccountInfo.owner.equals(TOKEN_2022_PROGRAM_ID) &&
            mintStateInfo &&
            authorityStateInfo
          ) {
            console.log(
              "All accounts already exist and initialized, skipping initialization"
            );
            return; // 所有账户都已初始化，可以跳过
          }

          try {
            // 检查mint账户和authorityState账户是否已经初始化
            const mintAccountInfo = await provider.connection.getAccountInfo(
              mintKeypair.publicKey
            );
            const authorityStateInfo = await provider.connection.getAccountInfo(
              authorityPda
            );
            const mintStateInfo = await provider.connection.getAccountInfo(
              mintStatePda
            );
            const pauseStateInfo = await provider.connection.getAccountInfo(
              pauseStatePda
            );

            // 如果所有账户都已初始化，则跳过初始化步骤
            if (
              mintAccountInfo &&
              mintAccountInfo.owner.equals(TOKEN_2022_PROGRAM_ID) &&
              authorityStateInfo &&
              mintStateInfo &&
              pauseStateInfo
            ) {
              console.log(
                "All accounts already initialized, skipping initialize step"
              );
              return;
            }

            // 如果mint账户未初始化，执行完整的初始化流程
            console.log("Performing full initialization...");

            // 修改：使用initialize_pda_only方法来初始化PDA账户
            // 因为mint账户已经在前面创建并初始化了
            try {
              // 使用程序的initialize_pda_only方法来初始化所有PDA账户
              const tx = await program.methods
                .initialize(6) // 6位小数
                .accounts({
                  authority: provider.wallet.publicKey,
                  authorityState: authorityPda,
                  tokenMint: mintKeypair.publicKey,
                  mintState: mintStatePda,
                  pauseState: pauseStatePda,
                  systemProgram: SystemProgram.programId,
                  tokenProgram: TOKEN_2022_PROGRAM_ID,
                  rent: anchor.web3.SYSVAR_RENT_PUBKEY,
                })
                .rpc();

              console.log(
                "PDA accounts initialized successfully with signature:",
                tx
              );

              // 等待交易确认
              await provider.connection.confirmTransaction(tx);
              await sleep(1000); // 等待一段时间确保账户更新

              // 验证账户是否已初始化
              const authorityStateInfo =
                await provider.connection.getAccountInfo(authorityPda);
              if (authorityStateInfo) {
                console.log("Authority state initialized successfully");
              } else {
                console.error("Authority state initialization failed");
              }
            } catch (error) {
              // 如果初始化失败，检查错误是否是因为账户已存在
              console.error("Error in initialization:", error);

              // 检查是否是因为账户已存在导致的错误
              if (error.toString().includes("already in use")) {
                console.log(
                  "Some accounts already exist. Proceeding with tests anyway."
                );
                // 继续执行测试，不抛出错误
              } else {
                // 其他错误，抛出异常
                throw error;
              }
            }
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
      // 使用已导入的getAssociatedTokenAddressSync函数
      recipientTokenAccount = getAssociatedTokenAddressSync(
        mintKeypair.publicKey,
        recipientKeypair.publicKey,
        false,
        TOKEN_2022_PROGRAM_ID // 确保使用TOKEN_2022_PROGRAM_ID
      );

      const createTokenAccountIx = createAssociatedTokenAccountInstruction(
        provider.wallet.publicKey,
        recipientTokenAccount,
        recipientKeypair.publicKey,
        mintKeypair.publicKey,
        TOKEN_2022_PROGRAM_ID // 确保使用TOKEN_2022_PROGRAM_ID
      );

      const tx = new anchor.web3.Transaction().add(createTokenAccountIx);
      const signature = await provider.sendAndConfirm(tx);
      await provider.connection.confirmTransaction(signature, "confirmed");
      await sleep(1000);
      console.log(
        "Recipient token account created:",
        recipientTokenAccount.toString()
      );
    } catch (error) {
      console.error("Token account creation failed:", error);
      throw error;
    }
  });

  it("Mint WUSD tokens", async () => {
    try {
      console.log("Debug mint operation:");
      console.log("Current Authority:", provider.wallet.publicKey.toString());

      // 执行铸币操作
      const tx = await program.methods
        .mint(new anchor.BN(10000000000), authorityBump)
        .accounts({
          authority: provider.wallet.publicKey,
          tokenMint: mintKeypair.publicKey,
          tokenAccount: recipientTokenAccount,
          tokenProgram: TOKEN_2022_PROGRAM_ID,
          authorityState: authorityPda,
          mintState: mintStatePda,
          pauseState: pauseStatePda,
        })
        .signers([provider.wallet.payer])
        .rpc();

      await provider.connection.confirmTransaction(tx);
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
          tokenMint: mintKeypair.publicKey,
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
      console.log("Recipient address:", recipientKeypair.publicKey.toString());

      // 为 spender 添加操作员权限
      const spender = Keypair.generate(); // 创建一个新的spender账户
      // 为spender账户和recipient账户转账一些SOL以支付账户创建费用
      console.log(
        "Transferring SOL to spender and recipient for account creation fees"
      );
      console.log("Spender address:", spender.publicKey.toString());

      // 转账SOL给spender账户
      const transferToSpenderTx = new anchor.web3.Transaction().add(
        SystemProgram.transfer({
          fromPubkey: provider.wallet.publicKey,
          toPubkey: spender.publicKey,
          lamports: LAMPORTS_PER_SOL * 0.1, // 转0.1 SOL
        })
      );
      await provider.sendAndConfirm(transferToSpenderTx);

      // 转账SOL给recipient账户
      const transferToRecipientTx = new anchor.web3.Transaction().add(
        SystemProgram.transfer({
          fromPubkey: provider.wallet.publicKey,
          toPubkey: recipientKeypair.publicKey,
          lamports: LAMPORTS_PER_SOL * 0.1, // 转0.1 SOL
        })
      );
      await provider.sendAndConfirm(transferToRecipientTx);

      console.log("SOL transferred to spender and recipient accounts");

      // 创建接收账户的代币账户
      const toTokenAccount = getAssociatedTokenAddressSync(
        mintKeypair.publicKey,
        spender.publicKey,
        false,
        TOKEN_2022_PROGRAM_ID
      );

      // 创建接收账户的代币账户
      const createTokenAccountIx = createAssociatedTokenAccountInstruction(
        provider.wallet.publicKey,
        toTokenAccount,
        spender.publicKey,
        mintKeypair.publicKey,
        TOKEN_2022_PROGRAM_ID
      );

      const tx = new anchor.web3.Transaction().add(createTokenAccountIx);
      const signature = await provider.sendAndConfirm(tx);
      await provider.connection.confirmTransaction(signature, "confirmed");
      console.log("Created spender token account");

      // 创建 permit 和 allowance 状态账户的 PDA
      const [allowanceStatePda] = PublicKey.findProgramAddressSync(
        [
          Buffer.from("allowance"),
          recipientKeypair.publicKey.toBuffer(),
          spender.publicKey.toBuffer(),
        ],
        program.programId
      );

      const [permitPda] = PublicKey.findProgramAddressSync(
        [
          Buffer.from("permit"),
          recipientKeypair.publicKey.toBuffer(),
          spender.publicKey.toBuffer(),
        ],
        program.programId
      );

      console.log("Debug PDA addresses:", {
        allowanceStatePda: allowanceStatePda.toString(),
        permitPda: permitPda.toString(),
        owner: recipientKeypair.publicKey.toString(),
        spender: spender.publicKey.toString(),
      });

      // 正确派生 freeze state PDAs
      const [fromFreezeState] = PublicKey.findProgramAddressSync(
        [Buffer.from("freeze"), recipientTokenAccount.toBuffer()],
        program.programId
      );

      const [toFreezeState] = PublicKey.findProgramAddressSync(
        [Buffer.from("freeze"), toTokenAccount.toBuffer()],
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
            tokenAccount: toTokenAccount,
            payer: provider.wallet.publicKey,
            systemProgram: SystemProgram.programId,
            tokenProgram: TOKEN_2022_PROGRAM_ID,
          })
          .rpc();

        await provider.connection.confirmTransaction(initToFreezeStateTx);
        console.log("Initialized to freeze state");
      }

      // 获取转账前的余额
      const balanceBefore = await provider.connection.getTokenAccountBalance(
        recipientTokenAccount
      );
      console.log(
        "Before transfer_from balance:",
        balanceBefore.value.uiAmount
      );

      // 创建permit授权
      const permitAmount = new anchor.BN(10000000); // 10 WUSD
      const currentTime = Math.floor(Date.now() / 1000);
      const deadline = currentTime + 3600; // 1小时后过期

      // 创建PermitScope对象
      const permitScope = {
        one_time: false,
        permanent: true,
        transfer: true,
        burn: false,
        all: false,
      };

      // 创建permit
      const permitTx = await program.methods
        .permit({
          amount: permitAmount,
          deadline: new anchor.BN(deadline),
          nonce: null,
          scope: permitScope,
          signature: new Uint8Array(64).fill(0),
          public_key: new Uint8Array(32).fill(0),
        })
        .accounts({
          owner: recipientKeypair.publicKey,
          spender: spender.publicKey,
          allowance: allowanceStatePda,
          permitState: permitPda,
          mintState: mintStatePda,
          tokenProgram: TOKEN_2022_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
          clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
        })
        .signers([recipientKeypair])
        .rpc();

      await provider.connection.confirmTransaction(permitTx);
      console.log("Permit created successfully");

      // 执行transfer_from操作
      const transferAmount = new anchor.BN(5000000); // 5 WUSD
      try {
        // 在执行transfer_from之前，先使用标准的SPL Token approve指令授权spender
        const approveIx = createApproveInstruction(
          recipientTokenAccount,
          spender.publicKey,
          recipientKeypair.publicKey,
          transferAmount.toNumber(),
          [],
          TOKEN_2022_PROGRAM_ID
        );

        // 先执行approve指令
        const approveTx = new anchor.web3.Transaction().add(approveIx);
        const approveSignature = await provider.connection.sendTransaction(
          approveTx,
          [recipientKeypair]
        );
        await provider.connection.confirmTransaction(
          approveSignature,
          "confirmed"
        );
        console.log("Approve transaction confirmed:", approveSignature);

        // 等待一段时间确保approve生效
        await sleep(2000);

        // 然后执行transfer_from
        const transferFromTx = await program.methods
          .transferFrom(transferAmount)
          .accounts({
            spender: spender.publicKey,
            owner: recipientKeypair.publicKey,
            fromToken: recipientTokenAccount,
            toToken: toTokenAccount,
            permit: permitPda,
            mintState: mintStatePda,
            pauseState: pauseStatePda,
            tokenProgram: TOKEN_2022_PROGRAM_ID,
            tokenMint: mintKeypair.publicKey,
            fromFreezeState: fromFreezeState,
            toFreezeState: toFreezeState,
            systemProgram: SystemProgram.programId,
          })
          .signers([spender]) // 只使用spender作为签名者
          .rpc();

        await provider.connection.confirmTransaction(
          transferFromTx,
          "confirmed"
        );
        console.log("Transfer_from executed successfully");

        // 验证转账结果
        const senderBalanceAfter =
          await provider.connection.getTokenAccountBalance(
            recipientTokenAccount
          );
        const receiverBalance =
          await provider.connection.getTokenAccountBalance(toTokenAccount);

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
      } catch (error) {
        console.error("Transfer_from operation failed:", error);
        throw error;
      }
    } catch (error) {
      console.error("Transfer_from failed:", error);
      throw error;
    }
  });

  it("Burn WUSD tokens", async () => {
    try {
      console.log("Starting burn test...");

      // 1. 获取销毁前的余额
      const balanceBefore = await provider.connection.getTokenAccountBalance(
        recipientTokenAccount
      );
      console.log("Balance before burn:", balanceBefore.value.uiAmount);

      // 2. 执行销毁操作，销毁50个WUSD代币
      const burnAmount = new anchor.BN(50000000);

      // 直接使用 program.methods 的 rpc() 方法发送交易
      const tx = await program.methods
        .burn(burnAmount)
        .accounts({
          authorityState: authorityPda,
          authority: recipientKeypair.publicKey,
          mint: mintKeypair.publicKey,
          tokenAccount: recipientTokenAccount,
          tokenProgram: TOKEN_2022_PROGRAM_ID,
          mintState: mintStatePda,
          pauseState: pauseStatePda,
        })
        .signers([recipientKeypair])
        .rpc();

      // 3. 等待交易确认
      await provider.connection.confirmTransaction(tx, "confirmed");
      console.log("Transaction confirmed:", tx);

      // 4. 验证销毁结果
      const balanceAfter = await provider.connection.getTokenAccountBalance(
        recipientTokenAccount
      );
      console.log("Balance after burn:", balanceAfter.value.uiAmount);

      // 5. 验证余额变化
      const expectedBalance =
        balanceBefore.value.uiAmount - burnAmount.toNumber() / 1000000;
      assert.approximately(
        balanceAfter.value.uiAmount,
        expectedBalance,
        0.000001,
        "Burn amount not correctly deducted"
      );

      console.log("Burn operation successful");
    } catch (error) {
      console.error("Burn operation failed:", error);
      throw error;
    }
  });
});
