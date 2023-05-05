Do NGINX part first, than continue here.

Just move this service to /etc/systemd/system/
run commands after:
     sudo systemctl daemon-reload
     sudo systemctl enable explorer
     sudo systemctl start explorer
