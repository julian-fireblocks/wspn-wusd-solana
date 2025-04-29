import {
    Keypair,
    LAMPORTS_PER_SOL,
    PublicKey,
    sendAndConfirmTransaction,
    Transaction,
    clusterApiUrl,
    SystemProgram,
    TransactionInstruction,
  } from "@solana/web3.js";
  
  import { TOKEN_2022_PROGRAM_ID } from "@solana/spl-token";
  
  import {
    FireblocksConnectionAdapter,
    FireblocksConnectionAdapterConfig,
    FeeLevel,
  } from "./fireblocks/index";
  import fs from "fs";
  
  require("dotenv").config();
  
  async function deployProgram() {
    console.log("Starting Solana program deployment...");
    console.log("apiKey:", process.env.FIREBLOCKS_API_KEY);
    console.log("apiSecretPath:", process.env.FIREBLOCKS_SECRET_KEY_PATH);
    console.log("vaultAccountId:", process.env.ADMIN_VAULT_ACCOUNT_ID);
  
    // Configure Fireblocks connection
    const fireblocksConnectionConfig: FireblocksConnectionAdapterConfig = {
      apiKey: process.env.FIREBLOCKS_API_KEY || "",
      apiSecretPath: process.env.FIREBLOCKS_SECRET_KEY_PATH || "",
      vaultAccountId: process.env.ADMIN_VAULT_ACCOUNT_ID || "",
      feeLevel: FeeLevel.HIGH, // Use high fee level for deployment transactions
      silent: false,
      devnet: true, // Set to true for devnet, false for mainnet
    };
  
    // Create connection to Solana devnet
    const connection = await FireblocksConnectionAdapter.create(
      clusterApiUrl("devnet"),
      fireblocksConnectionConfig
    );
  
    // Get the account public key from the Fireblocks vault
    const payerPublicKey = new PublicKey(connection.getAccount());
    console.log("Deployer account:", payerPublicKey.toBase58());
  
    // Create a new keypair for the program
    const programKeypair = Keypair.generate();
    console.log("Program ID:", programKeypair.publicKey.toBase58());
  
    // Create a new keypair for the program account
    const programAccountKeypair = Keypair.generate();
    console.log("Program Account:", programAccountKeypair.publicKey.toBase58());
  
    // Path to the compiled program (replace with your actual program path)
    // Note: You need to compile your Solana program using the Solana CLI before running this script
    // Example: solana-test-validator
    // In another terminal: cargo build-bpf --manifest-path=./path/to/program/Cargo.toml
    const programPath = process.env.FIREBLOCKS_DEPLOY_FILE || "";
  
    // For this example, we'll check if the file exists
    if (!fs.existsSync(programPath)) {
      console.error(`Program file not found at ${programPath}`);
      console.error(
        "Please compile your Solana program before running this script"
      );
      console.error(
        "Example: cargo build-bpf --manifest-path=./path/to/program/Cargo.toml"
      );
      return;
    }
  
    // Read the program file
    const programData = fs.readFileSync(programPath);
    console.log(`Program size: ${programData.length} bytes`);
  
    // Set transaction note
    connection.setTxNote(
      "Deploying Solana program with Fireblocks Connection Adapter"
    );
  
    try {
      // Calculate minimum balance for rent exemption
      const minimumBalanceForRentExemption =
        await connection.getMinimumBalanceForRentExemption(programData.length);
      console.log(
        `Minimum balance for rent exemption: ${minimumBalanceForRentExemption / LAMPORTS_PER_SOL} SOL`
      );
  
      // Deploy the program
      console.log("Deploying program...");
  
      // Note: BpfLoader.load requires a signer that we can't provide with Fireblocks
      // Instead, we'll use a multi-step approach with individual transactions
  
      // 1. Create program account
      const createAccountTransaction = new Transaction();
      createAccountTransaction.feePayer = payerPublicKey;
      createAccountTransaction.recentBlockhash = (
        await connection.getLatestBlockhash()
      ).blockhash;
      createAccountTransaction.add(
        SystemProgram.createAccount({
          fromPubkey: payerPublicKey,
          newAccountPubkey: programAccountKeypair.publicKey,
          lamports: minimumBalanceForRentExemption,
          space: programData.length,
          programId: TOKEN_2022_PROGRAM_ID,
        })
      );
  
      // Sign and send the create account transaction
      // Note: We need to sign with both the payer and the program account keypair
      // Since Fireblocks can only sign with the payer, we'll use partialSign for the program account keypair
      createAccountTransaction.partialSign(programAccountKeypair);
  
      // Serialize the transaction with verifySignatures set to false
      const serializedTx = createAccountTransaction.serialize({
        verifySignatures: false,
      });
      console.log("Serialized transaction:", serializedTx.toString("base64"));
  
      const createAccountTxHash = await sendAndConfirmTransaction(
        connection,
        createAccountTransaction,
        [] // Empty array since Fireblocks will handle the payer signature
      );
      console.log(
        `Program account created: https://explorer.solana.com/tx/${createAccountTxHash}?cluster=devnet`
      );
  
      // 2. Write program data in chunks
      console.log("Writing program data in chunks...");
  
      // Calculate optimal chunk size to ensure it doesn't exceed transaction size limit
      const calculateOptimalChunkSize = (dataLength: number) => {
        // Transaction size limit is 1232 bytes
        const MAX_TRANSACTION_SIZE = 1232;
        // Estimated transaction metadata size (including signatures, headers, etc.)
        // Increased metadata size estimate for safety
        const TRANSACTION_METADATA_SIZE = 400; // Increased to 400 to provide more safety margin
        // Instruction header size (instruction type 1 byte + offset 4 bytes)
        const INSTRUCTION_HEADER_SIZE = 5;
        // Additional safety margin
        const SAFETY_MARGIN = 150; // Increased safety margin
  
        // Calculate maximum bytes available for data
        const maxDataSize =
          MAX_TRANSACTION_SIZE -
          TRANSACTION_METADATA_SIZE -
          INSTRUCTION_HEADER_SIZE -
          SAFETY_MARGIN;
  
        // Set a very conservative initial chunk size
        let chunkSize = Math.min(maxDataSize, 600); // Reduced to 600 bytes
  
        // Calculate total chunks needed with current chunk size
        let totalChunks = Math.ceil(dataLength / chunkSize);
  
        return { chunkSize, totalChunks };
      };
  
      // Dynamically calculate optimal chunk size
      const { chunkSize, totalChunks } = calculateOptimalChunkSize(
        programData.length
      );
  
      console.log(`Optimal chunk size: ${chunkSize} bytes`);
      console.log(`Total chunks needed: ${totalChunks}`);
      let chunkTxHash = ""; // Declare variable outside loop
  
      for (let i = 0; i < totalChunks; i++) {
        const offset = i * chunkSize;
        let chunk = programData.slice(offset, offset + chunkSize);
  
        console.log(
          `Writing chunk ${i + 1}/${totalChunks} (${chunk.length} bytes) at offset ${offset}...`
        );
  
        const chunkTransaction = new Transaction();
        chunkTransaction.feePayer = payerPublicKey;
        chunkTransaction.recentBlockhash = (
          await connection.getLatestBlockhash()
        ).blockhash;
  
        // Optimize instruction data structure - use let declaration for possible later modifications
        let currentChunk = chunk;
        let instructionData = Buffer.concat([
          Buffer.from([0]), // Instruction (0 = Load)
          Buffer.from(new Uint32Array([offset]).buffer), // Write offset as 4 bytes
          currentChunk, // Chunk data
        ]);
  
        // Validate transaction size
        let estimatedSize = instructionData.length + 300; // Increase metadata estimate to ensure safety
  
        // If estimated size exceeds limit, gradually reduce current chunk until size requirement is met
        while (estimatedSize > 1232) {
          console.error(
            `Warning: Estimated transaction size (${estimatedSize}) approaching limit (1232), attempting to reduce chunk size`
          );
          // Reduce size by 10% each time
          currentChunk = currentChunk.slice(
            0,
            Math.floor(currentChunk.length * 0.9)
          );
          console.log(
            `Reducing current chunk size to ${currentChunk.length} bytes and retrying`
          );
  
          // Update instruction data
          instructionData = Buffer.concat([
            Buffer.from([0]), // Instruction (0 = Load)
            Buffer.from(new Uint32Array([offset]).buffer), // Write offset as 4 bytes
            currentChunk, // Reduced chunk data
          ]);
  
          // Recalculate estimated size
          estimatedSize = instructionData.length + 300;
  
          // If chunk is too small, it may not effectively transfer data, should throw an error
          if (currentChunk.length < 100) {
            throw new Error(
              `Unable to reduce chunk size to meet transaction size limit: current size ${estimatedSize}, limit 1232`
            );
          }
        }
  
        // If chunk size has been adjusted, record the final size
        if (currentChunk.length !== chunk.length) {
          console.log(
            `Adjusted chunk size from ${chunk.length} to ${currentChunk.length} bytes`
          );
          // Update chunk used in the loop to ensure correct offset calculation for subsequent chunks
          chunk = currentChunk;
        }
  
        chunkTransaction.add(
          new TransactionInstruction({
            programId: TOKEN_2022_PROGRAM_ID,
            keys: [
              {
                pubkey: programAccountKeypair.publicKey,
                isSigner: true,
                isWritable: true,
              },
            ],
            data: instructionData,
          })
        );
  
        try {
          console.log("Signing chunk transaction...");
          chunkTransaction.partialSign(programAccountKeypair);
  
          const chunkTxHash = await sendAndConfirmTransaction(
            connection,
            chunkTransaction,
            [] // Empty array since Fireblocks will handle the payer signature
          );
  
          console.log(
            `Chunk ${i + 1}/${totalChunks} written: https://explorer.solana.com/tx/${chunkTxHash}?cluster=devnet`
          );
        } catch (chunkError) {
          console.error(`Error writing chunk ${i + 1}:`, chunkError);
          throw chunkError;
        }
      }
  
      const writeTxHash = chunkTxHash; // Use last chunk's hash as the writeTxHash
      console.log(
        `Wrote entire program data: https://explorer.solana.com/tx/${writeTxHash}?cluster=devnet`
      );
  
      // 3. Finalize the program
      const finalizeTransaction = new Transaction();
      finalizeTransaction.feePayer = payerPublicKey;
      finalizeTransaction.recentBlockhash = (
        await connection.getLatestBlockhash()
      ).blockhash;
      // BpfLoader.finalize method doesn't exist, need to create custom instruction
      const finalizeData = Buffer.alloc(4);
      finalizeData.writeUInt32LE(1, 0); // Write instruction (1 = Finalize)
  
      finalizeTransaction.add(
        new TransactionInstruction({
          keys: [
            {
              pubkey: programAccountKeypair.publicKey,
              isSigner: true,
              isWritable: true,
            },
            {
              pubkey: new PublicKey(
                "SysvarRent111111111111111111111111111111111"
              ),
              isSigner: false,
              isWritable: false,
            },
          ],
          programId: TOKEN_2022_PROGRAM_ID,
          data: finalizeData,
        })
      );
  
      // Sign with program keypair since it's a required signer
      let finalizeTxHash;
      try {
        console.log("Signing finalize transaction...");
        finalizeTransaction.partialSign(programAccountKeypair);
        console.log("Finalize transaction signed successfully");
  
        finalizeTxHash = await sendAndConfirmTransaction(
          connection,
          finalizeTransaction,
          [] // Empty array since Fireblocks will handle the payer signature
        );
      } catch (finalizeSignError) {
        console.error("Error signing finalize transaction:", finalizeSignError);
        throw finalizeSignError;
      }
  
      console.log(
        `Program finalized: https://explorer.solana.com/tx/${finalizeTxHash}?cluster=devnet`
      );
      console.log("Program deployment completed successfully!");
      console.log(`Program ID: ${programKeypair.publicKey.toBase58()}`);
      console.log(
        `Verify your program at: https://explorer.solana.com/address/${programAccountKeypair.publicKey.toBase58()}?cluster=devnet`
      );
    } catch (error) {
      console.error("Error deploying program:", error);
    }
  }
  
  deployProgram();
  