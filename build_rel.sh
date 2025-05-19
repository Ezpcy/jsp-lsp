#!/bin/bash

cargo build --release

cp ./lombok.jar ./target/release/
