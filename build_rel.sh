#!/bin/bash

cargo build --release

cp ./lombok.jar ./target/release/
cp -r ./jdt-language-server ./target/release/
