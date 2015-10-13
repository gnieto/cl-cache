all: simple tagged

tagged:
	cd ../..; cargo build --release
	RUST_BACKTRACE=1 ../../target/release/warmup -y ../../extern/example/build.yml
	gcc -Wdeprecated-declarations -std=c99 tagged.c -o tagged -I /opt/AMDAPP/include/ -I../ -lOpenCL -L ../../target/release -lclcache -lm -lpthread -ldl

simple:
	cd ../..; cargo build --release
	gcc -Wdeprecated-declarations -std=c99 simple.c -o simple -I /opt/AMDAPP/include/ -I../ -lOpenCL -L ../../target/release -lclcache -lm -lpthread -ldl

clean:
	rm tagged simple