# indy-vdr
hyperledger indy client


To make changes to indy-vdr library and build it again, follow the given procedure:

1. Run cargo build --release
2. Then go to cd target/release
3. Copy the generated library "libindy_vdr.so" to the path /usr/local/lib
4. Add this lib folder to path: export LD_LIBRARY_PATH=/usr/local/lib:$LD_LIBRARY_PATH

Add Support for a new Plugin

1. 