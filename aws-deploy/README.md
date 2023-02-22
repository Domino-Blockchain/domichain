Usage:

```bash
./domichain_aws_setup.sh github_private_key_file  # Will reboot at the end
./domichain_bootstrap_validator.sh
hostname --ip-address  # Get private IP

# On the other AWS node
./domichain_aws_setup.sh github_private_key_file  # Will reboot at the end
./domichain_validator.sh 172.31.26.40  # private/public IP address of main RPC node (run "hostname --ip-address" on it)
```