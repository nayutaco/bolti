# bolti
WIP: A CLI tool for send/receive BOLT messages interactively

## Build
```sh
git clone https://github.com/nayutaco/bolti.git --recursive
cd bolti
make -C ptarmigan/libs
LIBCLANG_PATH=<where libclang.so exists> cargo build
```
