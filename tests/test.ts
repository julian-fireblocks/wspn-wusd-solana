import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { WusdToken } from "../target/types/wusd_token";
import * as spl from "@solana/spl-token";
import { TOKEN_2022_PROGRAM_ID } from "@solana/spl-token";
import { assert } from "chai";

describe("wusd-token", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.WusdToken as Program<WusdToken>;

  it("Initialize Contract", async () => {
    // 准备测试账户
    const admin = anchor.web3.Keypair.generate();
    const minter = anchor.web3.Keypair.generate();
    const pauser = anchor.web3.Keypair.generate();
    const tokenMint = anchor.web3.Keypair.generate();
    const decimals = 9;
    
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
    const [authorityState] = await anchor.web3.PublicKey.findProgramAddress(
      [Buffer.from("authority"), tokenMint.publicKey.toBuffer()],
      program.programId
    );

    // 创建mintState账户
    const [mintState] = await anchor.web3.PublicKey.findProgramAddress(
      [Buffer.from("mint_state"), tokenMint.publicKey.toBuffer()],
      program.programId
    );

    // 创建pauseState账户
    const [pauseState] = await anchor.web3.PublicKey.findProgramAddress(
      [Buffer.from("pause_state"), tokenMint.publicKey.toBuffer()],
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
  });
});
