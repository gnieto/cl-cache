all:
	cd ../..; cargo build --release
	RUST_BACKTRACE=1 ../../target/release/warmup -y ../../extern/simple/build.yml
	gcc -Wdeprecated-declarations -std=c99 main.c -o simple -I /opt/AMDAPP/include/ -I../ -lOpenCL -L ../../target/release -lclcache -lm -lpthread -ldl
