import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PumpFun } from '../target/types/pump_fun';
import NodeWallet from "@coral-xyz/anchor/dist/cjs/nodewallet";
import { Keypair, LAMPORTS_PER_SOL, PublicKey, SystemProgram, TransactionMessage, VersionedTransaction } from "@solana/web3.js";
import { BN } from "bn.js";
import { closeAccount, createMint, createSyncNativeInstruction, getOrCreateAssociatedTokenAccount, NATIVE_MINT, TOKEN_2022_PROGRAM_ID, TOKEN_PROGRAM_ID } from "@solana/spl-token";

describe("pumpfun", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const connection = provider.connection;
  const owner = provider.wallet as NodeWallet;
  const program = anchor.workspace.PumpFun as Program<PumpFun>;

  const user1 = Keypair.generate();
  const user2 = Keypair.generate();

  const appStats = PublicKey.findProgramAddressSync(
    [
      anchor.utils.bytes.utf8.encode("app-stats")
    ],
    program.programId
  )[0];

  const mintKeypair = Keypair.generate();
  const mint = mintKeypair.publicKey;
  const [authority, bump] = PublicKey.findProgramAddressSync(
    [
      anchor.utils.bytes.utf8.encode("authority"),
      mint.toBuffer()
    ],
    program.programId
  );
  const tokenAccountForPda = PublicKey.findProgramAddressSync(
    [
      anchor.utils.bytes.utf8.encode("token-account"),
      mint.toBuffer()
    ],
    program.programId
  )[0];
  const tokenCreate = PublicKey.findProgramAddressSync(
    [
      anchor.utils.bytes.utf8.encode("token-create"),
      mint.toBuffer()
    ],
    program.programId
  )[0];

  const feeAccount = owner.publicKey;
  const wsol = NATIVE_MINT;
  const nativeForPda = Keypair.generate();
  const tokenNativeForPda = nativeForPda.publicKey;
  const pair = PublicKey.findProgramAddressSync(
    [
      anchor.utils.bytes.utf8.encode("swap-pair"),
      mint.toBuffer()
    ],
    program.programId
  )[0];
  it("Setup", async () => {
    await connection.requestAirdrop(user1.publicKey, 5 * LAMPORTS_PER_SOL);
    await connection.requestAirdrop(user2.publicKey, 5 * LAMPORTS_PER_SOL);
  })


  // it("Is initialized!", async () => {
  //   // Add your test here.
  //   const tx = await program.methods.initialize(new BN(0.02 * LAMPORTS_PER_SOL)).accounts({
  //     owner: owner.publicKey,
  //     appStats,
  //     feeAccount: owner.publicKey
  //   }).rpc();
  //   console.log("Your transaction signature", tx);
  // });
  it("Create token", async () => {
    await createMint(
      connection,
      owner.payer,
      authority,
      authority,
      6,
      mintKeypair,
      undefined,
      TOKEN_2022_PROGRAM_ID
    );
    const tx = await program.methods.createToken(
      new BN(100000 * 10 ** 6),
      bump
    ).accounts({
      authority,
      mint,
      tokenAccountForPda,
      tokenCreate,
      feeAccount,
      appStats,
      tokenProgram: TOKEN_2022_PROGRAM_ID
    }).rpc().catch(e => console.log(e));
    console.log(tx);
  })
  it("linear price", async () => {
    console.log(authority.toBase58())
    const transferIxn = SystemProgram.transfer({
      fromPubkey: owner.publicKey,
      toPubkey: tokenNativeForPda,
      lamports: 2 * LAMPORTS_PER_SOL
    });
    const createAccountIxn = await program.methods.createAccount().accounts({
      mint: wsol,
      pda: authority,
      tokenAccount: tokenNativeForPda
    }).signers([nativeForPda]).instruction();
    const syncNativeIxn = createSyncNativeInstruction(tokenNativeForPda);
    let slope_numerator = new BN(1);
    let slope_denominator = new BN(200000000);
    let r0_numerator = new BN(150);  // since R and C both have 8 decimals, we don't need to do any scaling here (starts at 50 base RLY price for every 1 base CC)
    let r0_denominator = new BN(3);  // no
    const initLinearIxn = await program.methods.initializeLinearPrice(
      slope_numerator,
      slope_denominator,
      r0_numerator,
      r0_denominator,
      bump,
    ).accounts({
      pair,
      mint,
      wsol,
      pda: authority,
      tokenForPda: tokenAccountForPda,
      tokenNativeForPda,
      tokenProgramMint: TOKEN_2022_PROGRAM_ID,
      tokenProgram: TOKEN_PROGRAM_ID
    }).instruction();
    let { blockhash } = await connection.getLatestBlockhash();
    const instructions = [createAccountIxn, transferIxn, syncNativeIxn, initLinearIxn];
    // Create the transaction message
    const message = new TransactionMessage({
      payerKey: owner.publicKey, // Public key of the account that will pay for the transaction
      recentBlockhash: blockhash, // Latest blockhash
      instructions, // Instructions included in transaction
    }).compileToV0Message();
    const transaction = new VersionedTransaction(message);
    transaction.sign([owner.payer, nativeForPda]);
    const txn = await connection.sendTransaction(transaction);
    console.log(txn);
  });
  it ("swap to token", async () => {
    const tokenAccountForSwapper = await getOrCreateAssociatedTokenAccount(
      connection,
      owner.payer,
      mint,
      owner.publicKey,
      undefined,
      undefined,
      undefined,
      TOKEN_2022_PROGRAM_ID
    );
    const nativeAccountForSwapper = await getOrCreateAssociatedTokenAccount(
      connection,
      owner.payer,
      NATIVE_MINT,
      owner.publicKey
    );
    const transferIxn = SystemProgram.transfer({
      fromPubkey: owner.publicKey,
      toPubkey: nativeAccountForSwapper.address,
      lamports: LAMPORTS_PER_SOL
    });
    const syncNativeIxn = createSyncNativeInstruction(nativeAccountForSwapper.address);
    const swapIxn = await program.methods.swapToToken(new BN(LAMPORTS_PER_SOL)).accounts({
      pda: authority,
      mint,
      wsol,
      pair,
      tokenAccountForSwapper: tokenAccountForSwapper.address,
      nativeAccountForSwapper: nativeAccountForSwapper.address,
      tokenAccountForPda,
      nativeAccountForPda: tokenNativeForPda,
      tokenProgramMint: TOKEN_2022_PROGRAM_ID
    }).instruction();
    let { blockhash } = await connection.getLatestBlockhash();
    const instructions = [transferIxn, syncNativeIxn, swapIxn];
    // Create the transaction message
    const message = new TransactionMessage({
      payerKey: owner.publicKey, // Public key of the account that will pay for the transaction
      recentBlockhash: blockhash, // Latest blockhash
      instructions, // Instructions included in transaction
    }).compileToV0Message();
    const transaction = new VersionedTransaction(message);
    transaction.sign([owner.payer])
    const txn = await connection.sendTransaction(transaction);
    console.log(txn);
  });
  it ("swap to sol", async () => {
    const tokenAccountForSwapper = await getOrCreateAssociatedTokenAccount(
      connection,
      owner.payer,
      mint,
      owner.publicKey,
      undefined,
      undefined,
      undefined,
      TOKEN_2022_PROGRAM_ID
    );
    const nativeAccountForSwapper = await getOrCreateAssociatedTokenAccount(
      connection,
      owner.payer,
      NATIVE_MINT,
      owner.publicKey
    );
    const swapIxn = await program.methods.swapToSol(new BN(10)).accounts({
      pda: authority,
      mint,
      wsol,
      pair,
      tokenAccountForSwapper: tokenAccountForSwapper.address,
      nativeAccountForSwapper: nativeAccountForSwapper.address,
      tokenAccountForPda,
      nativeAccountForPda: tokenNativeForPda,
      tokenProgramMint: TOKEN_2022_PROGRAM_ID
    }).instruction();
    let { blockhash } = await connection.getLatestBlockhash();
    const instructions = [swapIxn];
    // Create the transaction message
    const message = new TransactionMessage({
      payerKey: owner.publicKey, // Public key of the account that will pay for the transaction
      recentBlockhash: blockhash, // Latest blockhash
      instructions, // Instructions included in transaction
    }).compileToV0Message();
    const transaction = new VersionedTransaction(message);
    transaction.sign([owner.payer])
    const txn = await connection.sendTransaction(transaction);
    console.log(txn);
    await closeAccount(
      connection,
      owner.payer,
      nativeAccountForSwapper.address,
      owner.publicKey,
      owner.publicKey
    );
  })
});
