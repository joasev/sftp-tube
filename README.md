# SFTP Tube

SFTP Tube is a specialized SFTP client designed to enhance operational efficiency by using an SFTP server strictly as a transient transport layer. It facilitates the secure transfer of data between systems while mitigating the risks associated with long-term file storage on SFTP servers - a common vector for ransomware attacks due to poor usage hygiene.

## Features

- **Temporary Directories and Automatic Cleanup**: This approach leverages pre-defined temporary directories for file transfers, ensuring that data resides on the server for minimal time. By automatically removing files upon download, the SFTP server is transformed into a "tube" that merely passes data rather than storing it. This minimizes the risk of attacks or ransomware exploiting data persistently stored on the SFTP server, as the data's exposure time is drastically minimized. 
- **Restricted Copy/Paste**: Emulates a one-line-at-a-time copy/paste mechanism through the SFTP server. This feature enables environments to block traditional methods like RDP copy/paste to prevent uncontrolled data flow, while still allowing a streamlined method for transferring small amounts of data for operational efficiency.

## Configuration

SFTP Tube is configured via a `st-config.txt` file, which specifies the SFTP server details and default directory paths. Ensure this file is correctly set up before running the executable. The configuration file should include:

- **IP and Port**: The IP address and port number of your SFTP server.
- **Default Folders**: Paths for the root and temporary directories used during file transfers.

## Usage

To use SFTP Tube for file transfers:

1. **Place the Executable**: Copy the compiled executable to all endpoints involved in the data transfer.
2. **Run SFTP Tube**: Double-click the executable to launch. Follow the on-screen instructions to initiate file transfers or use the one-line copy/paste functionality.
3. **Interact as Prompted**: The user-friendly interface will guide you through the necessary steps for secure data transfer.

