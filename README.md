# rust-wget

Reimplementation of wget in rust.

Supported options are:

```
    -h, --help          Show this help menu
    -c, --continue      Continue getting a partially-downloaded file
    -S, --server-response 
                        Print the headers sent by HTTP servers
    -t, --tries NUMBER  Set number of tries to NUMBER. Specify 0 for infinite
                        retrying.
    -T, --timeout SECONDS
                        Set the network timeout to SECONDS seconds
        --backups BACKUPS
                        Before (over)writing a file, back up an existing file
                        by adding a .1 suffix to the file name. Such backup
                        files are rotated to .2, .3, and so on, up to BACKUPS
                        (and lost beyond that).
        --user USER     Specify the username for file retrieval
        --password PASSWORD
                        Specify the password for file retrieval
        --ask-password  Prompt for a password instead of using explicit
                        password with password option
        --header HEADER-LINE
                        Send header-line along with the rest of the headers in
                        each HTTP request
```

- supports http and https, no ftp
- follows redirects
- accepts multiple urls for one run
- tracks and displays progress
