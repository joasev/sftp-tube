use std::{
    error::Error,
    fmt,
    fs::{self, File},
    io::{self, BufRead, copy, prelude::*},
    net::TcpStream,
    path::Path,
    process,
};
use rpassword::prompt_password;
use ssh2::{Session, Sftp};


const CONFIG_FILE_NAME: &str = "st-config.txt";
const LOCAL_TO_THROW_DIR: &str = "To Upload";
const LOCAL_CAUGHT_DIR: &str = "Downloaded";

fn main() -> io::Result<()> {
    

    println!("");
    println!("*********************************************");
    println!("***************** SFTP Tube *****************");
    println!("*********************************************");

    let config_path = Path::new(CONFIG_FILE_NAME);
    let config = match load_config(config_path) {
        Ok(config) => {
            println!("Configuration loaded successfully from {}!", CONFIG_FILE_NAME);
            println!("");
            config
        }
        Err(e) => {
            println!("Failed to load configuration. {}", e);
            println!("Please ensure the configuration file is formatted correctly and located at '{}'", config_path.display());
            // Process exit instead of panic to avoid verbose inforamtion for end user
            process::exit(1);
        }
    };

    // Connect to the SSH server
    let tcp = TcpStream::connect(config.ip_and_port)?;
    let mut sess = Session::new()?;
    sess.set_tcp_stream(tcp);
    sess.handshake()?;

    let username = request_user_input("Enter SFTP Username: ", false)?;
    let credential = request_user_input("Enter Password: ", true)?;

    sess.userauth_password(&username, &credential)?;
    println!("Authenticated: {}",sess.authenticated());
    let mut sftp = sess.sftp()?;

    // Create temp folders in SFTP if not available
    let folder_path = config.root_folder_name + "/" + config.temp_folder_name.as_str() + "/";
    sftp.mkdir(Path::new(&folder_path), 0o777).ok();
    let folder_path = folder_path + &username;
    sftp.mkdir(Path::new(&folder_path), 0o777).ok();

    loop {
        println!("");
        println!("Main Menu. What do you need to do?");
        println!("1. SFTP Clipboard. Copy and paste SINGLE lines of text through SFTP.");
        println!("2. SFTP Throw. Throw files in the \"Throw this\" folder to SFTP at your default folder.");
        println!("3. SFTP Catch. Retrieve files from your default folder in SFTP and place them in your local folder \"Caught\". Clears thrown files in SFTP.");
        println!("e. Exit.");
        let choice = request_user_input("Please enter your choice: ", false)?;

        match choice.as_str() {
            "1" => {
                sftp = resume_session(&sess, &username, &credential)?.unwrap_or(sftp);
                clipboard_loop(&sftp, &folder_path, &config.temp_file_name)?}
            "2" => {
                sftp = resume_session(&sess, &username, &credential)?.unwrap_or(sftp);
                throw_files(&sftp, &folder_path)?}
            "3" => {
                sftp = resume_session(&sess, &username, &credential)?.unwrap_or(sftp);
                fetch_and_clean_files(&sftp, &folder_path)?}
            "e" | "E" | "exit" | "EXIT" => break,
            _ => println!("Invalid choice, please try again."),
        }
    }

    Ok(())

}

fn resume_session(sess: &Session, username: &str, credential: &str) -> Result<Option<Sftp>, ssh2::Error> {
    match sess.authenticated() {
        false => {
            sess.userauth_password(username, credential)?;
            println!("Resumed session: {}", sess.authenticated());
            sess.sftp().map(Some)
        },
        true => Ok(None),
    }
}

fn throw_files(sftp: &Sftp, remote_dir: &str) -> io::Result<()> {
    if fs::read_dir(LOCAL_TO_THROW_DIR)?.next().is_none() {
        println!("");
        println!("Folder \"{}\" is empty! No files were uploaded.", LOCAL_TO_THROW_DIR);
    }
    // Iterate over the contents of the local directory
    for entry in fs::read_dir(LOCAL_TO_THROW_DIR)? {
        let entry = entry?;
        let path = entry.path();

        // Only proceed if the entry is a file
        if path.is_file() {
            println!("");
            println!("Uploading: {}", path.to_str().unwrap());

            // Construct the remote file path
            let remote_file_path = format!("{}/{}", remote_dir, path.file_name().unwrap().to_str().unwrap());

            // Open the local file
            let mut local_file = fs::File::open(&path)?;

            // Create or truncate the remote file
            let mut remote_file = sftp.create(Path::new(&remote_file_path))?;

            // Copy contents of the local file to the remote file
            io::copy(&mut local_file, &mut remote_file)?;

            println!("Uploaded: {}", path.to_str().unwrap());
        }
    }

    Ok(())
}

fn fetch_and_clean_files(sftp: &Sftp, remote_dir: &str) -> io::Result<()> {
    let full_remote_path = "/".to_string() + remote_dir;
    
    // List the contents of the remote directory
    let readdir = sftp.readdir(Path::new(&full_remote_path))
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    
    if readdir.is_empty() {
        println!("");
        println!("Remote directory \"{}\" is empty! No files to download.", remote_dir);
        return Ok(());
    }

    for (path, stat) in readdir {
        
        // Only proceed if the entry is a file
        if stat.is_file() {
            
            let file_name = path.file_name().unwrap().to_str().unwrap(); // Safe to unwrap here based on the nature of SFTP
            let local_file_path = format!("{}/{}", LOCAL_CAUGHT_DIR, file_name);
            println!("");
            println!("Downloading: {}", file_name);

            // Download the remote file
            let mut remote_file = sftp.open(Path::new(path.as_path())).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
            let mut local_file = File::create(&local_file_path)?;
            copy(&mut remote_file, &mut local_file)?;

            println!("Downloaded: {}", file_name);

            // Delete the remote file after successful download
            sftp.unlink(Path::new(path.as_path())).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
            println!("Deleted remote file: {}/{}", full_remote_path, file_name);
        }
    }

    Ok(())
}

fn clipboard_loop(sftp: &Sftp, folder_path: &str, temp_file_name: &str) -> io::Result<()> {
    
    let file_path = folder_path.to_owned() + "/" + temp_file_name;

    println!("");
    loop {
        let choice = request_user_input("Copy text TO Sftp or FROM Sftp or Exit. (t/f/exit): ", false)?;
        let choice = choice.to_uppercase();
        match choice.as_str() {
            "T" | "TO" => {
                let payload = request_user_input("Enter text: ", false)?;

                sftp.create(&Path::new(&file_path))?
                .write_all(payload.as_bytes())?;
            
                println!("");
                println!("Text: {}", payload);
                println!("Copied to sftp-clipboard!");
                println!("");
            }
            "F" | "FROM" => {
                let file_path = Path::new(&file_path);
                if let Ok(mut file) = sftp.open(file_path) {

                    let mut contents = String::new();
                    file.read_to_string(&mut contents)?;
    
                    println!("");
                    println!("Text: {}", contents);
                    println!("Retrieved from Sftp-clipboard");
    
                    // Using the `contents` string read from the SFTP file
                    match clipboard_win::set_clipboard_string(&contents) {
                        Ok(_) => println!("Copied to your windows clipboard!"),
                        Err(_) => println!("Could not copy to your windows clipboard"),
                    }
                    println!("");
                    
                    // Delete the remote temp file after successful download
                    sftp.unlink(Path::new(file_path)).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
                
                } else {
                    println!("");
                    println!("No temp clipboard data in SFTP.");
                    println!("NOTE that you can only retrieve SFTP-clipboard text ONCE because temp data is cleared at retrieval!");
                    println!("Try copying TO Sftp again.");
                    println!("");
                }
                
            }
            "E" | "EXIT" => break,
            _ => println!("Invalid choice, please try again."),
        }
    }
    Ok(())
    
}


#[derive(Debug)]
struct Config {
    ip_and_port: String,
    root_folder_name: String,
    temp_folder_name: String,
    temp_file_name: String,
}

#[derive(Debug)]
struct ConfigError {
    msg: String,
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Configuration Error: {}", self.msg)
    }
}

impl Error for ConfigError {}

fn load_config(file_path: &Path) -> Result<Config, Box<dyn Error>> {
    let mut ip_and_port = None;
    let mut root_folder_name = None;
    let mut temp_folder_name = None;
    let mut temp_file_name = None;

    let file = File::open(file_path)?;
    let lines = io::BufReader::new(file).lines();

    for line in lines {
        let line = line?;
        let parts: Vec<&str> = line.splitn(2, ":").collect();
        if parts.len() == 2 {
            let key = parts[0].trim();
            let value = parts[1].trim();
            match key {
                "IP and port" => ip_and_port = Some(value.to_string()),
                "Root folder name" => root_folder_name = Some(value.to_string()),
                "Temp folder name" => temp_folder_name = Some(value.to_string()),
                "Temp file name" => temp_file_name = Some(value.to_string()),
                _ => {},
            }
        }
    }

    Ok(Config {
        ip_and_port: ip_and_port.ok_or_else(|| ConfigError { msg: "IP and port not found".to_string() })?,
        root_folder_name: root_folder_name.ok_or_else(|| ConfigError { msg: "Root folder name not found".to_string() })?,
        temp_folder_name: temp_folder_name.ok_or_else(|| ConfigError { msg: "Temp folder name not found".to_string() })?,
        temp_file_name: temp_file_name.ok_or_else(|| ConfigError { msg: "Temp file name not found".to_string() })?,
    })
}




fn request_user_input(message: &str, sensitive: bool) -> io::Result<String> {
    if sensitive {
        // For sensitive information like passwords, use prompt_password
        prompt_password(message)
    } else {
        // For non-sensitive information, use the original method
        print!("{}", message);
        io::stdout().flush().unwrap(); // Make sure the prompt is displayed

        let mut user_input = String::new();
        io::stdin().read_line(&mut user_input)?;
        Ok(user_input.trim().to_string()) // Remove newline
    }
}