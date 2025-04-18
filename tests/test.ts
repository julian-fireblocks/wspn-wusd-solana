import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { WusdToken } from "../target/types/wusd_token";
import * as spl from "@solana/spl-token";
import { TOKEN_2022_PROGRAM_ID } from "@solana/spl-token";
import { assert } from "chai";

describe("wusd-token", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const minter = anchor.web3.Keypair.generate();
  const pauser = anchor.web3.Keypair.generate();
  const program = anchor.workspace.WusdToken as Program<WusdToken>;

  // 从deploy-keypair.json导入本地账号
  let localWallet: anchor.web3.Keypair;

  before(async () => {
    // 从deploy-keypair.json文件中读取密钥
    const keypairData = require("../deploy-keypair.json");
    const localWalletBytes = new Uint8Array(keypairData);
    localWallet = anchor.web3.Keypair.fromSecretKey(localWalletBytes);
    console.log("Local wallet:", localWallet.publicKey.toBase58());

    // 从本地账号转账SOL给minter和pauser账户

    // 转账给minter
    const transferToMinter = new anchor.web3.Transaction().add(
      anchor.web3.SystemProgram.transfer({
        fromPubkey: localWallet.publicKey,
        toPubkey: minter.publicKey,
        lamports: anchor.web3.LAMPORTS_PER_SOL * 10,
      })
    );
    await provider.connection.sendTransaction(transferToMinter, [localWallet]);

    // 转账给pauser
    const transferToPauser = new anchor.web3.Transaction().add(
      anchor.web3.SystemProgram.transfer({
        fromPubkey: localWallet.publicKey,
        toPubkey: pauser.publicKey,
        lamports: anchor.web3.LAMPORTS_PER_SOL * 10,
      })
    );
    await provider.connection.sendTransaction(transferToPauser, [localWallet]);

    await new Promise((resolve) => setTimeout(resolve, 2000));
  });
  // 共享变量
  const admin = anchor.web3.Keypair.generate();
  const tokenMint = anchor.web3.Keypair.generate();
  const decimals = 9;
  const recipient = anchor.web3.Keypair.generate();
  const amount = new anchor.BN(1000000000000); // 1000 WUSD
  let recipientTokenAccount: anchor.web3.PublicKey;
  let authorityState: anchor.web3.PublicKey;
  let mintState: anchor.web3.PublicKey;
  let pauseState: anchor.web3.PublicKey;
  let freezeState: anchor.web3.PublicKey;
  let authorityBump: number;

  it("Initialize Contract", async () => {
    // 转账给admin
    const transferToAdmin = new anchor.web3.Transaction().add(
      anchor.web3.SystemProgram.transfer({
        fromPubkey: localWallet.publicKey,
        toPubkey: admin.publicKey,
        lamports: anchor.web3.LAMPORTS_PER_SOL * 10,
      })
    );
    await provider.connection.sendTransaction(transferToAdmin, [localWallet]);

    // 等待资金到账
    await new Promise((resolve) => setTimeout(resolve, 3000));
    console.log("Admin", admin.publicKey.toBase58());
    console.log("ProgramId", program.programId.toBase58());

    // 创建并初始化token_mint账户
    await spl.createMint(
      provider.connection,
      admin,
      admin.publicKey,
      admin.publicKey,
      decimals,
      tokenMint,
      { commitment: "confirmed" },
      TOKEN_2022_PROGRAM_ID
    );

    // 等待token_mint账户初始化完成
    await new Promise((resolve) => setTimeout(resolve, 2000));
    // 创建authorityState账户
    [authorityState, authorityBump] =
      await anchor.web3.PublicKey.findProgramAddress(
        [Buffer.from("authority"), tokenMint.publicKey.toBuffer()],
        program.programId
      );

    // 创建mintState账户
    [mintState] = await anchor.web3.PublicKey.findProgramAddress(
      [Buffer.from("mint_state"), tokenMint.publicKey.toBuffer()],
      program.programId
    );

    // 创建pauseState账户
    [pauseState] = await anchor.web3.PublicKey.findProgramAddress(
      [Buffer.from("pause_state"), tokenMint.publicKey.toBuffer()],
      program.programId
    );

    // 创建接收者的token账户
    recipientTokenAccount = await spl.createAccount(
      provider.connection,
      admin,
      tokenMint.publicKey,
      admin.publicKey,
      undefined,
      { commitment: "confirmed" },
      TOKEN_2022_PROGRAM_ID
    );

    // 创建freezeState账户
    [freezeState] = await anchor.web3.PublicKey.findProgramAddress(
      [
        Buffer.from("freeze"),
        recipientTokenAccount.toBuffer(),
        tokenMint.publicKey.toBuffer(),
      ],
      program.programId
    );

    // 调用initialize方法
    await program.methods
      .initialize(decimals)
      .accounts({
        authority: admin.publicKey,
        minter: minter.publicKey,
        pauser: pauser.publicKey,
        tokenMint: tokenMint.publicKey,
        authorityState: authorityState,
        mintState: mintState,
        pauseState: pauseState,
        tokenProgram: TOKEN_2022_PROGRAM_ID,
        systemProgram: anchor.web3.SystemProgram.programId,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      })
      .signers([admin])
      .rpc();

    const authorityStateAccount = await program.account.authorityState.fetch(
      authorityState
    );

    // 验证状态账户
    assert(
      authorityStateAccount.admin.equals(admin.publicKey),
      "Admin pubkey mismatch"
    );
    assert(
      authorityStateAccount.minterRoles[0].equals(minter.publicKey),
      "Minter role mismatch"
    );
    assert(
      authorityStateAccount.pauserRoles[0].equals(pauser.publicKey),
      "Pauser role mismatch"
    );

    const mintStateAccount = await program.account.mintState.fetch(mintState);
    assert(
      mintStateAccount.mint.equals(tokenMint.publicKey),
      "Mint pubkey mismatch"
    );
    assert(mintStateAccount.decimals === decimals, "Decimals mismatch");

    // 初始化freezeState账户
    await program.methods
      .initializeFreezeState()
      .accounts({
        authority: admin.publicKey,
        authorityState: authorityState,
        tokenMint: tokenMint.publicKey,
        freezeState: freezeState,
        tokenAccount: recipientTokenAccount,
        payer: admin.publicKey,
        tokenProgram: TOKEN_2022_PROGRAM_ID,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([admin])
      .rpc();
    console.log("Contract initialized successfully");
  });

  it("Mint WUSD", async () => {
    // 从本地账号转账SOL给接收者账户
    const transferToRecipient = new anchor.web3.Transaction().add(
      anchor.web3.SystemProgram.transfer({
        fromPubkey: localWallet.publicKey,
        toPubkey: recipient.publicKey,
        lamports: anchor.web3.LAMPORTS_PER_SOL * 10,
      })
    );
    await provider.connection.sendTransaction(transferToRecipient, [
      localWallet,
    ]);
    await new Promise((resolve) => setTimeout(resolve, 2000));

    // 创建接收者的token账户
    recipientTokenAccount = await spl.createAccount(
      provider.connection,
      recipient,
      tokenMint.publicKey,
      recipient.publicKey,
      undefined,
      { commitment: "confirmed" },
      TOKEN_2022_PROGRAM_ID
    );

    // 创建freezeState账户
    [freezeState] = await anchor.web3.PublicKey.findProgramAddress(
      [
        Buffer.from("freeze"),
        recipientTokenAccount.toBuffer(),
        tokenMint.publicKey.toBuffer(),
      ],
      program.programId
    );

    // 初始化freezeState账户
    await program.methods
      .initializeFreezeState()
      .accounts({
        authority: admin.publicKey,
        authorityState: authorityState,
        tokenMint: tokenMint.publicKey,
        freezeState: freezeState,
        tokenAccount: recipientTokenAccount,
        payer: admin.publicKey,
        tokenProgram: TOKEN_2022_PROGRAM_ID,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([admin])
      .rpc();

    // 调用mint方法
    console.log(
      "Attempting to mint",
      amount.div(new anchor.BN(10 ** decimals)).toString(),
      "WUSD"
    );
    await program.methods
      .mint(amount, authorityBump)
      .accounts({
        authority: minter.publicKey,
        tokenMint: tokenMint.publicKey,
        tokenAccount: recipientTokenAccount,
        authorityState: authorityState,
        mintState: mintState,
        pauseState: pauseState,
        freezeState: freezeState,
        tokenProgram: TOKEN_2022_PROGRAM_ID,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([minter])
      .rpc();

    // 验证铸币结果
    const tokenAccountInfo = await spl.getAccount(
      provider.connection,
      recipientTokenAccount,
      undefined,
      TOKEN_2022_PROGRAM_ID
    );

    // 将实际铸币数量转换为WUSD单位
    const mintedAmount = new anchor.BN(tokenAccountInfo.amount.toString());
    const mintedWUSD = mintedAmount
      .div(new anchor.BN(10 ** decimals))
      .toString();
    console.log("Successfully minted", mintedWUSD, "WUSD to recipient");

    assert(mintedAmount.eq(amount), "Amount mismatch");
  });

  it("Transfer WUSD", async () => {
    // 创建转账目标账户
    const transferAmount = new anchor.BN(100000000000); // 100 WUSD
    const transferRecipient = anchor.web3.Keypair.generate();

    // 从本地账号转账SOL给转账目标账户
    // 转账给transferRecipient
    const transferToTransferRecipient = new anchor.web3.Transaction().add(
      anchor.web3.SystemProgram.transfer({
        fromPubkey: localWallet.publicKey,
        toPubkey: transferRecipient.publicKey,
        lamports: anchor.web3.LAMPORTS_PER_SOL * 10,
      })
    );
    await provider.connection.sendTransaction(transferToTransferRecipient, [
      localWallet,
    ]);
    await new Promise((resolve) => setTimeout(resolve, 2000));

    // 创建转账目标的token账户
    const transferRecipientTokenAccount = await spl.createAccount(
      provider.connection,
      transferRecipient,
      tokenMint.publicKey,
      transferRecipient.publicKey,
      undefined,
      { commitment: "confirmed" },
      TOKEN_2022_PROGRAM_ID
    );
    // freezer 角色设置，用于测试冻结账户功能
    const freezer = anchor.web3.Keypair.generate();
    // 从本地账号转账SOL给freezer账户
    const transferToFreezer = new anchor.web3.Transaction().add(
      anchor.web3.SystemProgram.transfer({
        fromPubkey: localWallet.publicKey,
        toPubkey: freezer.publicKey,
        lamports: anchor.web3.LAMPORTS_PER_SOL * 10,
      })
    );
    await provider.connection.sendTransaction(transferToFreezer, [localWallet]);
    await new Promise((resolve) => setTimeout(resolve, 2000));

    // 创建目标账户的freezeState
    const [transferFreezeState] =
      await anchor.web3.PublicKey.findProgramAddress(
        [
          Buffer.from("freeze"),
          transferRecipientTokenAccount.toBuffer(),
          tokenMint.publicKey.toBuffer(),
        ],
        program.programId
      );

    // 初始化目标账户的freezeState
    await program.methods
      .initializeFreezeState()
      .accounts({
        authority: admin.publicKey,
        authorityState: authorityState,
        tokenMint: tokenMint.publicKey,
        freezeState: transferFreezeState,
        tokenAccount: transferRecipientTokenAccount,
        payer: admin.publicKey,
        tokenProgram: TOKEN_2022_PROGRAM_ID,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([admin])
      .rpc(); 

    // 执行转账
    await program.methods
      .transfer(transferAmount)
      .accounts({
        from: recipient.publicKey,
        fromToken: recipientTokenAccount,
        to: transferRecipient.publicKey,
        toToken: transferRecipientTokenAccount,
        tokenMint: tokenMint.publicKey,
        authorityState: authorityState,
        pauseState: pauseState,
        fromFreezeState: freezeState,
        toFreezeState: transferFreezeState,
        tokenProgram: TOKEN_2022_PROGRAM_ID,
      })
      .signers([recipient])
      .rpc();

    // 验证转账结果
    const fromTokenAccountInfo = await spl.getAccount(
      provider.connection,
      recipientTokenAccount,
      undefined,
      TOKEN_2022_PROGRAM_ID
    );

    const toTokenAccountInfo = await spl.getAccount(
      provider.connection,
      transferRecipientTokenAccount,
      undefined,
      TOKEN_2022_PROGRAM_ID
    );

    // 验证转账金额
    const expectedFromBalance = amount.sub(transferAmount);
    assert(
      new anchor.BN(fromTokenAccountInfo.amount.toString()).eq(
        expectedFromBalance
      ),
      "From account amount mismatch"
    );
    assert(
      new anchor.BN(toTokenAccountInfo.amount.toString()).eq(transferAmount),
      "To account amount mismatch"
    );

    // 输出转账后的账户余额
    const fromBalance = new anchor.BN(fromTokenAccountInfo.amount.toString())
      .div(new anchor.BN(10 ** decimals))
      .toString();
    const toBalance = new anchor.BN(toTokenAccountInfo.amount.toString())
      .div(new anchor.BN(10 ** decimals))
      .toString();
    console.log("From account balance:", fromBalance, "WUSD");
    console.log("To account balance:", toBalance, "WUSD");
    console.log("Transfer completed successfully");
  });

  it("Transfer From WUSD", async () => {
    // 创建委托账户
    const delegate = anchor.web3.Keypair.generate();
    // 从本地账号转账SOL给委托账户
    const transferToDelegate = new anchor.web3.Transaction().add(
      anchor.web3.SystemProgram.transfer({
        fromPubkey: localWallet.publicKey,
        toPubkey: delegate.publicKey,
        lamports: anchor.web3.LAMPORTS_PER_SOL * 10,
      })
    );
    await provider.connection.sendTransaction(transferToDelegate, [
      localWallet,
    ]);
    await new Promise((resolve) => setTimeout(resolve, 2000));

    // 创建转账目标账户
    const transferAmount = new anchor.BN(50000000000); // 50 WUSD
    const transferRecipient = anchor.web3.Keypair.generate();

    // 从本地账号转账SOL给转账目标账户
    // 转账给transferRecipient
    const transferToTransferRecipient = new anchor.web3.Transaction().add(
      anchor.web3.SystemProgram.transfer({
        fromPubkey: localWallet.publicKey,
        toPubkey: transferRecipient.publicKey,
        lamports: anchor.web3.LAMPORTS_PER_SOL * 10,
      })
    );
    await provider.connection.sendTransaction(transferToTransferRecipient, [
      localWallet,
    ]);
    await new Promise((resolve) => setTimeout(resolve, 2000));

    // 创建转账目标的token账户
    const transferRecipientTokenAccount = await spl.createAccount(
      provider.connection,
      transferRecipient,
      tokenMint.publicKey,
      transferRecipient.publicKey,
      undefined,
      { commitment: "confirmed" },
      TOKEN_2022_PROGRAM_ID
    );

    // 创建目标账户的freezeState
    const [transferFreezeState] =
      await anchor.web3.PublicKey.findProgramAddress(
        [
          Buffer.from("freeze"),
          transferRecipientTokenAccount.toBuffer(),
          tokenMint.publicKey.toBuffer(),
        ],
        program.programId
      );

    // 初始化目标账户的freezeState
    await program.methods
      .initializeFreezeState()
      .accounts({
        authority: admin.publicKey,
        authorityState: authorityState,
        tokenMint: tokenMint.publicKey,
        freezeState: transferFreezeState,
        tokenAccount: transferRecipientTokenAccount,
        payer: admin.publicKey,
        tokenProgram: TOKEN_2022_PROGRAM_ID,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([admin])
      .rpc();

    // 创建permit_state账户
    const [permitState] = await anchor.web3.PublicKey.findProgramAddress(
      [
        Buffer.from("permit"),
        recipient.publicKey.toBuffer(),
        delegate.publicKey.toBuffer(),
      ],
      program.programId
    );

    // 设置授权过期时间为1小时后
    const currentTime = Math.floor(Date.now() / 1000);
    const expiryTime = currentTime + 3600;

    // 执行授权操作
    await program.methods
      .approve(transferAmount, new anchor.BN(expiryTime))
      .accounts({
        owner: recipient.publicKey,
        delegate: delegate.publicKey,
        tokenAccount: recipientTokenAccount,
        permitState: permitState,
        tokenMint: tokenMint.publicKey,
        mintState: mintState,
        pauseState: pauseState,
        tokenProgram: TOKEN_2022_PROGRAM_ID,
        systemProgram: anchor.web3.SystemProgram.programId,
        clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
      })
      .signers([recipient])
      .rpc();

    // 执行transfer_from操作
    await program.methods
      .transferFrom(transferAmount)
      .accounts({
        owner: recipient.publicKey,
        spender: delegate.publicKey,
        fromToken: recipientTokenAccount,
        toToken: transferRecipientTokenAccount,
        tokenMint: tokenMint.publicKey,
        mintState: mintState,
        pauseState: pauseState,
        permit: permitState,
        fromFreezeState: freezeState,
        toFreezeState: transferFreezeState,
        tokenProgram: TOKEN_2022_PROGRAM_ID,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([delegate])
      .rpc();

    // 验证转账结果
    const fromTokenAccountInfo = await spl.getAccount(
      provider.connection,
      recipientTokenAccount,
      undefined,
      TOKEN_2022_PROGRAM_ID
    );

    const toTokenAccountInfo = await spl.getAccount(
      provider.connection,
      transferRecipientTokenAccount,
      undefined,
      TOKEN_2022_PROGRAM_ID
    );

    // 验证转账金额
    const previousTransferAmount = new anchor.BN(100000000000); // 100 WUSD from previous transfer
    const expectedBalance = amount
      .sub(previousTransferAmount)
      .sub(transferAmount);
    assert(
      new anchor.BN(fromTokenAccountInfo.amount.toString()).eq(expectedBalance),
      "From account amount mismatch"
    );
    assert(
      new anchor.BN(toTokenAccountInfo.amount.toString()).eq(transferAmount),
      "To account amount mismatch"
    );

    // 输出转账后的账户余额
    const fromBalance = new anchor.BN(fromTokenAccountInfo.amount.toString())
      .div(new anchor.BN(10 ** decimals))
      .toString();
    const toBalance = new anchor.BN(toTokenAccountInfo.amount.toString())
      .div(new anchor.BN(10 ** decimals))
      .toString();
    console.log("From account balance:", fromBalance, "WUSD");
    console.log("To account balance:", toBalance, "WUSD");
    console.log("Transfer From completed successfully");
  });

  it("Burn WUSD", async () => {
    // 创建burner账户
    const burner = anchor.web3.Keypair.generate();
    // 从本地账号转账SOL给burner账户
    const transferToBurner = new anchor.web3.Transaction().add(
      anchor.web3.SystemProgram.transfer({
        fromPubkey: localWallet.publicKey,
        toPubkey: burner.publicKey,
        lamports: anchor.web3.LAMPORTS_PER_SOL * 10,
      })
    );
    await provider.connection.sendTransaction(transferToBurner, [localWallet]);
    await new Promise((resolve) => setTimeout(resolve, 2000));

    // 创建burner的token账户
    const burnerTokenAccount = await spl.createAccount(
      provider.connection,
      burner,
      tokenMint.publicKey,
      burner.publicKey,
      undefined,
      { commitment: "confirmed" },
      TOKEN_2022_PROGRAM_ID
    );

    // 创建burner的freezeState
    const [burnerFreezeState] = await anchor.web3.PublicKey.findProgramAddress(
      [
        Buffer.from("freeze"),
        burnerTokenAccount.toBuffer(),
        tokenMint.publicKey.toBuffer(),
      ],
      program.programId
    );

    // 初始化burner的freezeState
    await program.methods
      .initializeFreezeState()
      .accounts({
        authority: admin.publicKey,
        authorityState: authorityState,
        tokenMint: tokenMint.publicKey,
        freezeState: burnerFreezeState,
        tokenAccount: burnerTokenAccount,
        payer: admin.publicKey,
        tokenProgram: TOKEN_2022_PROGRAM_ID,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([admin])
      .rpc();

    // 设置Burner角色
    await program.methods
      .setRole({ burner: {} }, burner.publicKey, true)
      .accounts({
        admin: admin.publicKey,
        pauseState: pauseState,
        authorityState: authorityState,
        tokenMint: tokenMint.publicKey,
      })
      .signers([admin])
      .rpc();
    await new Promise((resolve) => setTimeout(resolve, 2000));

    // 直接mint 100 WUSD到burner账户
    const mintAmount = new anchor.BN(100000000000); // 100 WUSD
    await program.methods
      .mint(mintAmount, authorityBump)
      .accounts({
        authority: minter.publicKey,
        tokenMint: tokenMint.publicKey,
        tokenAccount: burnerTokenAccount,
        authorityState: authorityState,
        mintState: mintState,
        pauseState: pauseState,
        freezeState: burnerFreezeState,
        tokenProgram: TOKEN_2022_PROGRAM_ID,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([minter])
      .rpc();

    // 获取burner账户余额
    const beforeBurnBalance = await spl.getAccount(
      provider.connection,
      burnerTokenAccount,
      undefined,
      TOKEN_2022_PROGRAM_ID
    );

    // 输出burn前的账户余额
    const beforeBurnWUSD = new anchor.BN(beforeBurnBalance.amount.toString())
      .div(new anchor.BN(10 ** decimals))
      .toString();
    console.log("Burner account balance before burn:", beforeBurnWUSD, "WUSD");

    // 执行burn操作
    await program.methods
      .burn(mintAmount)
      .accounts({
        authority: burner.publicKey,
        mint: tokenMint.publicKey,
        tokenAccount: burnerTokenAccount,
        authorityState: authorityState,
        mintState: mintState,
        pauseState: pauseState,
        freezeState: burnerFreezeState,
        tokenProgram: TOKEN_2022_PROGRAM_ID,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([burner])
      .rpc();

    // 验证burn后的账户余额
    const afterBurnBalance = await spl.getAccount(
      provider.connection,
      burnerTokenAccount,
      undefined,
      TOKEN_2022_PROGRAM_ID
    );

    // 验证余额
    assert(
      new anchor.BN(afterBurnBalance.amount.toString()).eq(new anchor.BN(0)),
      "Balance after burn mismatch"
    );

    // 输出burn后的账户余额
    const finalBalance = new anchor.BN(afterBurnBalance.amount.toString())
      .div(new anchor.BN(10 ** decimals))
      .toString();
    console.log("Burner account balance after burn:", finalBalance, "WUSD");
    console.log("Burn completed successfully");
  });
});
