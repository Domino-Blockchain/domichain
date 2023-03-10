Usage:

```bash
# On your PC
# Copy shared identity files
scp -i "domichain-23-02-06.pem" id_rsa id_rsa.pub ubuntu@ec2-18-189-189-145.us-east-2.compute.amazonaws.com:~/.ssh/
# Copy deploy scripts
scp -i "~/domichain-23-02-06.pem" domichain_aws_setup.sh domichain_bootstrap_validator.sh domichain_validator.sh ubuntu@ec2-18-189-189-145.us-east-2.compute.amazonaws.com:~
```

```bash
chmod +x ./domichain_*.sh
screen -S setup
./domichain_aws_setup.sh ~/.ssh/id_rsa name-of-git-branch  # Will reboot at the end
./domichain_bootstrap_validator.sh
hostname -I | cut -d' ' -f1  # Get private IP

# On the other AWS node
chmod +x ./domichain_*.sh
./domichain_aws_setup.sh ~/.ssh/id_rsa name-of-git-branch  # Will reboot at the end
./domichain_validator.sh 172.31.26.40  # private/public IP address of main RPC node (run "hostname -I | cut -d' ' -f1" on it)
```

```bash
# For ping rpc-url:
target/release/domichain-gossip --allow-private-addr rpc-url --timeout 10 --entrypoint 127.0.0.1:8001
```