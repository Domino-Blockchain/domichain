Firstly install nginx  
$ sudo apt install nginx  
than certbot  
$ sudo apt install certbot &&  
$ sudo apt install python3-certbot-nginx  
    
after that copy explorer and node into /etc/nginx/sites-enabled, and remove default  

run commands in same directory:  
$ sudo certbot --nginx # and follow instructions from the script.  
$ sudo systemctl daemon-reload  
$ sudo systemctl enable nginx # this enables script to autostart after reboot  
$ sudo systemctl start nginx # this starts nginx  
    
first command will modify explorer and node a bit, adding 443 port support and pointers to its certificates.  
