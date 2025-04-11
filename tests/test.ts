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
  let freezeBump: number;

  it("Initialize Contract", async () => {
    // 为管理员账户提供资金
    await provider.connection.requestAirdrop(
      admin.publicKey,
      anchor.web3.LAMPORTS_PER_SOL * 1
    );
    // 等待资金到账
    await new Promise(resolve => setTimeout(resolve, 2000));
    console.log("admin", admin.publicKey.toBase58());
    console.log("programId", program.programId.toBase58());
    
    // 创建并初始化token_mint账户
    await spl.createMint(
      provider.connection,
      admin,
      admin.publicKey,
      admin.publicKey,
      decimals,
      tokenMint,
      { commitment: 'confirmed' },
      TOKEN_2022_PROGRAM_ID
    );

    // 等待token_mint账户初始化完成
    await new Promise(resolve => setTimeout(resolve, 2000));

    // 创建authorityState账户
    [authorityState, authorityBump] = await anchor.web3.PublicKey.findProgramAddress(
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
      { commitment: 'confirmed' },
      TOKEN_2022_PROGRAM_ID
    );

    // 创建freezeState账户
    [freezeState, freezeBump] = await anchor.web3.PublicKey.findProgramAddress(
      [Buffer.from("freeze"), recipientTokenAccount.toBuffer(), tokenMint.publicKey.toBuffer()],
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
    assert(authorityStateAccount.admin.equals(admin.publicKey), "Admin pubkey mismatch");
    assert(authorityStateAccount.minterRoles[0].equals(minter.publicKey), "Minter role mismatch");
    assert(authorityStateAccount.pauserRoles[0].equals(pauser.publicKey), "Pauser role mismatch");

    const mintStateAccount = await program.account.mintState.fetch(mintState);
    assert(mintStateAccount.mint.equals(tokenMint.publicKey), "Mint pubkey mismatch");
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
    // 为接收者账户提供资金
    await provider.connection.requestAirdrop(
      recipient.publicKey,
      anchor.web3.LAMPORTS_PER_SOL * 1
    );
    await new Promise(resolve => setTimeout(resolve, 2000));
    
    // 创建接收者的token账户
    recipientTokenAccount = await spl.createAccount(
      provider.connection,
      recipient,
      tokenMint.publicKey,
      recipient.publicKey,
      undefined,
      { commitment: 'confirmed' },
      TOKEN_2022_PROGRAM_ID
    );

    // 创建freezeState账户
    [freezeState, freezeBump] = await anchor.web3.PublicKey.findProgramAddress(
      [Buffer.from("freeze"), recipientTokenAccount.toBuffer(), tokenMint.publicKey.toBuffer()],
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
    console.log("Attempting to mint", amount.div(new anchor.BN(10 ** decimals)).toString(), "WUSD");
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
    const mintedWUSD = mintedAmount.div(new anchor.BN(10 ** decimals)).toString();
    console.log("Successfully minted", mintedWUSD, "WUSD to recipient");

    assert(mintedAmount.eq(amount), "Amount mismatch");
  });
});
