use tokio;
use std::str::FromStr;
use std::io::{self, Write};
use namada_sdk::{
    MaybeSend, 
    MaybeSync,
    args::{InputAmount, TxTransparentTransferData, TxBuilder}, 
    io::{StdIo, Io, Client}, 
    masp::{ShieldedUtils, fs::FsShieldedUtils}, 
    wallet::{WalletIo, DerivationPath, WalletStorage, fs::FsWalletUtils}, 
    Namada, 
    NamadaImpl, 
    chain::ChainId,
    zeroize::Zeroizing,
    bip39::Mnemonic,
    key::{SchemeType},
    rpc, // Importing RPC for account info
};
use namada_core::address::Address;
use namada_sdk::signing::default_sign;
use namada_sdk::ExtendedViewingKey;
use namada_sdk::PaymentAddress;
use rand_core::OsRng;
use namada_sdk::masp::find_valid_diversifier;
use namada_core::key::common::CommonPublicKey;
use tendermint_rpc::{HttpClient, Url};
use std::error::Error;
use toml::Value;
use tokio::fs;

const RPC_URL: &str = "https://rpc.knowable.run:443"; // RPC URL
const CHAIN_ID: &str = "housefire-reduce.e51ecf4264fc3"; // Chain ID
const OWNER_ADDRESS: &str = "tnam1qze5x6au3egfnq7qp963c793cev5z5jvkcufnfhj"; // Just a placeholder to check if address is reveal pk or not

#[tokio::main]
async fn main() {
    let url = Url::from_str(RPC_URL).expect("Invalid RPC address");
    let http_client = HttpClient::new(url).expect("Failed to create HTTP client");

    let wallet = FsWalletUtils::new("./sdk-wallet".into());
    let shielded_ctx = FsShieldedUtils::new("./masp".into());
    let std_io = StdIo;

    let sdk = NamadaImpl::new(http_client, wallet, shielded_ctx, std_io)
        .await
        .expect("Unable to initialize Namada context")
        .chain_id(ChainId::from_str(CHAIN_ID).expect("Invalid chain ID"));

    // Load existing wallet
    if sdk.wallet_mut().await.load().is_ok() {
        println!("Existing wallet found");
    } else {
        println!("No existing wallet found.");
    }

    loop {
        display_menu();

        let choice = get_user_choice();
        match choice {
            1 => create_wallet(&sdk).await,
            2 => add_key(&sdk).await,
            3 => print_address(&sdk).await,
            4 => create_spending_key(&sdk).await,
            5 => generate_payment_address(&sdk).await,
            6 => send_token(&sdk).await, 
            7 => check_if_revealed(&sdk).await, // New option to check if account is revealed
            8 => {
                println!("Exiting...");
                break;
            }
            _ => println!("Invalid choice, please enter a valid option."),
        }
    }
}
// Display menu options
fn display_menu() {
    println!("\nNamada wallet example:");
    println!("1. Create a new wallet");
    println!("2. Add a new key from a mnemonic");
    println!("3. Print an address from the wallet");
    println!("4. Create a spending key");
    println!("5. Generate a payment address");
    println!("6. Send tokens");
    println!("7. Check if account is revealed"); 
    println!("8. Exit");
}
// User input here
fn get_user_choice() -> usize {
    print!("Enter your choice: ");
    io::stdout().flush().expect("Failed to flush stdout");

    let mut input = String::new();
    io::stdin().read_line(&mut input).expect("Failed to read line");

    input.trim().parse::<usize>().unwrap_or(0) // Default to 0 if parsing fails
}

// Create a new wallet
async fn create_wallet<C, U, V, I>(sdk: &NamadaImpl<C, U, V, I>)
where
    C: Client + MaybeSync + MaybeSend,
    U: WalletIo + WalletStorage + MaybeSync + MaybeSend,
    V: ShieldedUtils + MaybeSync + MaybeSend,
    I: Io + MaybeSync + MaybeSend,
{
    let mnemonic = Mnemonic::new(namada_sdk::bip39::MnemonicType::Words24, namada_sdk::bip39::Language::English);
    let phrase = mnemonic.phrase();

    println!("Generated mnemonic: {}", phrase);
    let alias = prompt_user("Enter an alias for the new wallet: ");

    let derivation_path = DerivationPath::default_for_transparent_scheme(SchemeType::Ed25519);
    let (_key_alias, _sk) = sdk.wallet_mut().await
        .derive_store_key_from_mnemonic_code(
            SchemeType::Ed25519,
            Some(alias),
            true,
            derivation_path,
            Some((mnemonic.clone(), Zeroizing::new("".to_owned()))),
            true,
            None,
        ).expect("Unable to derive key from mnemonic code");

    sdk.wallet().await.save().expect("Could not save wallet!");
    println!("Wallet created and saved!");
}

// Add a key from a mnemonic
async fn add_key<C, U, V, I>(sdk: &NamadaImpl<C, U, V, I>)
where
    C: Client + MaybeSync + MaybeSend,
    U: WalletIo + WalletStorage + MaybeSync + MaybeSend,
    V: ShieldedUtils + MaybeSync + MaybeSend,
    I: Io + MaybeSync + MaybeSend,
{
    let phrase = prompt_user("Enter the mnemonic: ");
    let alias = prompt_user("Enter an alias: ");
    let mnemonic = Mnemonic::from_phrase(&phrase, namada_sdk::bip39::Language::English).expect("Invalid mnemonic");

    let derivation_path = DerivationPath::default_for_transparent_scheme(SchemeType::Ed25519);
    let (_key_alias, _sk) = sdk.wallet_mut().await
        .derive_store_key_from_mnemonic_code(
            SchemeType::Ed25519,
            Some(alias),
            true,
            derivation_path,
            Some((mnemonic.clone(), Zeroizing::new("".to_owned()))),
            true,
            None,
        ).expect("Unable to derive key from mnemonic code");

    sdk.wallet().await.save().expect("Could not save wallet!");
    println!("Key added successfully!");
}

// Print an address associated with an alias
async fn print_address<C, U, V, I>(sdk: &NamadaImpl<C, U, V, I>)
where
    C: Client + MaybeSync + MaybeSend,
    U: WalletIo + WalletStorage + MaybeSync + MaybeSend,
    V: ShieldedUtils + MaybeSync + MaybeSend,
    I: Io + MaybeSync + MaybeSend,
{
    let alias = prompt_user("Which alias do you want to look up? ");
    match sdk.wallet().await.find_address(&alias) {
        Some(address) => println!("Address for {}: {:?}", alias, address),
        None => println!("No address found for alias: {}", alias),
    }
}

// Create a new spending key
async fn create_spending_key<C, U, V, I>(sdk: &NamadaImpl<C, U, V, I>)
where
    C: Client + MaybeSync + MaybeSend,
    U: WalletIo + WalletStorage + MaybeSync + MaybeSend,
    V: ShieldedUtils + MaybeSync + MaybeSend,
    I: Io + MaybeSync + MaybeSend,
{
    let phrase = prompt_user("Enter the mnemonic for the spending key: ");
    let spending_alias = prompt_user("Enter an alias for the spending key: ");
    let mnemonic = Mnemonic::from_phrase(&phrase, namada_sdk::bip39::Language::English).expect("Invalid mnemonic");

    let spending_derivation_path = DerivationPath::default_for_shielded();
    let (_spending_key_alias, sk_spending) = sdk.wallet_mut().await
        .derive_store_spending_key_from_mnemonic_code(
            spending_alias.clone(),
            true,
            None,
            spending_derivation_path,
            Some((mnemonic.clone(), Zeroizing::new("".to_owned()))),
            true,
            None,
        ).expect("Unable to derive spending key from mnemonic");

    println!("Derived spending key: {:?}", sk_spending);
    sdk.wallet().await.save().expect("Could not save wallet!");
    println!("Spending key created and saved!");
}

// Generate a shielded payment address
async fn generate_payment_address<C, U, V, I>(sdk: &NamadaImpl<C, U, V, I>)
where
    C: Client + MaybeSync + MaybeSend,
    U: WalletIo + WalletStorage + MaybeSync + MaybeSend,
    V: ShieldedUtils + MaybeSync + MaybeSend,
    I: Io + MaybeSync + MaybeSend,
{
    // Hardcoded viewing key
    let viewing_key_str = "zvknam1qddsrtp4qqqqpqr6t24a76wu3gdszc0jw8r0643mhfs3sgx49cftd8qjtetl4a5aa24fmryf29uz7xkqket0exqm8vkky8w99uqjl80cl290uqfev3yegg3ym4z84x5gwruuw4t2ln26wkadckksfkfu8ku6jdqjryvdvtlq3x8atu9p3lk7a86wals57zp7dfnydr8088pmflt6c2zgwjnzdnrsfy4v3r85gf2my2ynzqtug4euewsj0ps6upqrw524jw5g5cyecjq4c8gjy";
    let viewing_key = ExtendedViewingKey::from_str(viewing_key_str).expect("Invalid viewing key");

    // Hardcoded alias
    let alias = "default"; // No need for user input
    let alias_force = true; // Set force to true

    // Check if an address already exists for the alias
    if let Some(address) = sdk.wallet().await.find_address(&alias) {
        println!("Address already exists for {}: {:?}", alias, address);

        if !alias_force {
            println!("Skipping address generation for {}, as alias exists and force option is not enabled.", alias);
            return; // Exit if not forcing alias generation
        } else {
            println!("Forcing address generation for {}...", alias);
        }
    } else {
        println!("No address found for alias: {}, generating new payment address...", alias);
    }

    // Use the provided viewing key
    let viewing_key_ref = &viewing_key.as_viewing_key();

    // Generate the shielded payment address
    let (div, _g_d) = find_valid_diversifier(&mut OsRng);
    let masp_payment_addr = viewing_key_ref
        .to_payment_address(div)
        .expect("Unable to generate a PaymentAddress");
    let payment_addr = PaymentAddress::from(masp_payment_addr);

    // Store the payment address in the wallet
    sdk.wallet_mut().await
        .insert_payment_addr(alias.to_string(), payment_addr.clone(), alias_force) // Convert alias to String
        .expect("Payment address could not be inserted");

    // Save the wallet with the new address
    sdk.wallet().await.save().expect("Could not save wallet!");

    println!("New payment address generated and saved for {}: {:?}", alias, payment_addr);
}

// New function to send tokens
async fn send_token<C, U, V, I>(sdk: &NamadaImpl<C, U, V, I>)
where
    C: Client + MaybeSync + MaybeSend,
    U: WalletIo + WalletStorage + MaybeSync + MaybeSend,
    V: ShieldedUtils + MaybeSync + MaybeSend,
    I: Io + MaybeSync + MaybeSend,
{
    let alias = "rilsso-public";
    let tendermint_addr = "https://rpc.knowable.run:443"; 

    // Retrieve the source address using the alias
    let source_address = match sdk.wallet().await.find_address(&alias) {
        Some(address) => address.into_owned(),
        None => {
            println!("No address found for alias: {}", alias);
            return;
        }
    };

    // Check if the account is already revealed
    if !findifreveal(sdk, tendermint_addr, &source_address)
        .await
        .expect("Error checking reveal status")
    {
        println!("Account is not revealed, proceeding to reveal the public key.");

        // Create a new reveal transaction builder for the public key
        let _test = get_viewing_keys().await; 

        match _test {
            Ok(keys) => {
                for key in keys {
                    println!("{}", key); 
                }
            }
            Err(e) => {
                eprintln!("Error getting viewing keys: {}", e);
            }
        }

        // Fetch public keys from the wallet.toml
        let _testpk = get_public_keys().await; 
        
        match _testpk {
            Ok(keys) => {
                for key in keys {
                    println!("{}", key); 
                    
                    // Pass the derived key to the `CommonPublicKey::from_str`
                    let public_key = CommonPublicKey::from_str(&key)
                        .expect("Invalid public key format");

                    let reveal_tx_builder = sdk
                        .new_reveal_pk(public_key.clone())
                        .signing_keys(vec![public_key.clone()]);

                    // Build the reveal transaction
                    let (mut reveal_tx, signing_data) = reveal_tx_builder
                        .build(sdk)
                        .await
                        .expect("Unable to build reveal pk tx");

                    // Sign the reveal transaction
                    sdk.sign(&mut reveal_tx, &reveal_tx_builder.tx, signing_data, default_sign, ())
                        .await
                        .expect("Unable to sign reveal pk tx");

                    // Submit the signed reveal transaction
                    match sdk.submit(reveal_tx.clone(), &reveal_tx_builder.tx).await {
                        Ok(res) => println!("Public key successfully revealed: {:?}", res),
                        Err(e) => {
                            println!("Failed to reveal public key: {:?}", e);
                            return;  // Exit if revealing the public key fails
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("Error getting public keys: {}", e);
            }
        }
    } else {
        println!("Account is already revealed, skipping the reveal step.");
    }


    let target_address = Address::from_str("tnam1qqzg5khvcfdgnjg4wghvxcnekxwu4kg5nuwjssjt")
        .expect("Invalid target address");
    let amount = InputAmount::from_str("1").expect("Invalid amount");

    let token = sdk.native_token();

    // Prepare the transaction data
    let data = TxTransparentTransferData {
        source: source_address,
        target: target_address,
        token,
        amount,
    };

    // Fetch public keys again for use in signing the transfer transaction
    let _testpk = get_public_keys().await; 
    let signing_keys: Vec<CommonPublicKey> = match _testpk {
        Ok(keys) => keys.iter()
            .map(|key| CommonPublicKey::from_str(key).expect("Invalid public key format"))
            .collect(),
        Err(e) => {
            eprintln!("Error getting public keys: {}", e);
            return; 
        }
    };

    let mut transfer_tx_builder = sdk
        .new_transparent_transfer(vec![data])
        .signing_keys(signing_keys);

    // Build and sign the transaction
    let (mut transfer_tx, signing_data) = transfer_tx_builder
        .build(sdk)
        .await
        .expect("Unable to build transfer");

    // Sign the transaction
    sdk.sign(&mut transfer_tx, &transfer_tx_builder.tx, signing_data, default_sign, ())
        .await
        .expect("Unable to sign transparent-transfer tx");

    // Submit the signed transaction to the ledger
    match sdk.submit(transfer_tx, &transfer_tx_builder.tx).await {
        Ok(res) => println!("Transaction successfully submitted: {:?}", res),
        Err(e) => println!("Failed to submit transaction: {:?}", e),
    }
}


// Check revealed or not
async fn check_if_revealed<C, U, V, I>(sdk: &NamadaImpl<C, U, V, I>)
where
    C: Client + MaybeSync + MaybeSend,
    U: WalletIo + WalletStorage + MaybeSync + MaybeSend,
    V: ShieldedUtils + MaybeSync + MaybeSend,
    I: Io + MaybeSync + MaybeSend,
{
    let owner_address = Address::from_str(OWNER_ADDRESS).expect("Invalid owner address");

    match findifreveal(sdk, RPC_URL, &owner_address).await {
        Ok(is_revealed) => {
            if is_revealed {
                println!("The account is revealed.");
            } else {
                println!("The account is not revealed.");
            }
        }
        Err(e) => eprintln!("Error checking reveal status: {}", e),
    }
}

// Function to check if an account is revealed by querying the Tendermint node
async fn findifreveal<C, U, V, I>( // Custom function so _sdk 
    _sdk: &NamadaImpl<C, U, V, I>,
    tendermint_addr: &str,
    owner: &Address,
) -> Result<bool, Box<dyn Error>>
where
    C: Client + MaybeSync + MaybeSend,
    U: WalletIo + WalletStorage + MaybeSync + MaybeSend,
    V: ShieldedUtils + MaybeSync + MaybeSend,
    I: Io + MaybeSync + MaybeSend,
{
    let client = HttpClient::new(
        Url::from_str(tendermint_addr)
            .map_err(|e| Box::new(e) as Box<dyn Error>)?,
    )?;

    let account_info: Option<namada_sdk::account::Account> = rpc::get_account_info(&client, owner).await?;
    if let Some(account) = account_info {
        println!("Account information: {:?}", account);
        Ok(!account.public_keys_map.idx_to_pk.is_empty()) // Return true if public keys exist
    } else {
        println!("No account information found.");
        Ok(false)
    }
}


async fn get_viewing_keys() -> Result<Vec<String>, String> {
    let file_path = "./sdk-wallet/wallet.toml"; 

    // Await the future and use map_err on the result
    let content = fs::read_to_string(file_path).await.map_err(|e| format!("Unable to read file: {}", e))?;

    let parsed: Value = toml::de::from_str(&content).map_err(|e| format!("Unable to parse TOML: {}", e))?;

    let mut keys = Vec::new();
    if let Some(view_keys) = parsed.get("view_keys") {
        for (_key, value) in view_keys.as_table().unwrap() {
            if let Some(address) = value.get("key") {
                keys.push(clean_address(address.as_str().unwrap()));
            }
        }
    } else {
        return Err("No view_keys found.".to_string());
    }

    Ok(keys)
}

fn clean_address(address: &str) -> String {
    address.trim().to_string()
}



async fn get_public_keys() -> Result<Vec<String>, String> {
    let file_path = "./sdk-wallet/wallet.toml"; 

    let content = fs::read_to_string(file_path).await.map_err(|e| format!("Unable to read file: {}", e))?;

    let parsed: Value = toml::de::from_str(&content).map_err(|e| format!("Unable to parse TOML: {}", e))?;

    let mut keys = Vec::new();
    if let Some(public_keys) = parsed.get("public_keys") {
        for (_key, value) in public_keys.as_table().unwrap() {
            if let Some(address) = value.as_str() {
                // Remove the "ED25519_PK_PREFIX" prefix from the address
                let cleaned_address = address.replace("ED25519_PK_PREFIX", "");
                keys.push(cleaned_address);
            }
        }
    } else {
        return Err("No public_keys found.".to_string());
    }

    Ok(keys)
}


fn prompt_user(prompt: &str) -> String {
    print!("{}", prompt);
    io::stdout().flush().expect("Failed to flush stdout");

    let mut input = String::new();
    io::stdin().read_line(&mut input).expect("Failed to read line");
    input.trim().to_string()
}
