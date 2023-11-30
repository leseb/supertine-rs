# supertini Project

This small project helped me getting a little bit into the Rust programming language.
The program will run a given binary and watch for any changes on it. If the file changes, the program will kill the running process and start it again.

You can build the go binary in the `src`  directory. This binary will just run forever and print a
random string every seconds.

```
$ go build -o src/bin
```

Then you can run the `supertini` program with the binary as argument.

```bash
$ cargo build
$ ./target/debug/supertini --binary-path wait
[2023-11-30T10:47:15Z INFO  supertini] running command 'wait '
2023/11/30 11:47:15 2023-11-30 11:47:15.837702434 +0100 CET m=+0.000049969 1480460F-538A-15F2-4D14-8D64C4399FDF
2023/11/30 11:47:16 2023-11-30 11:47:16.837897107 +0100 CET m=+1.000244649 1480460F-538A-15F2-4D14-8D64C4399FDF
2023/11/30 11:47:17 2023-11-30 11:47:17.838007431 +0100 CET m=+2.000355031 1480460F-538A-15F2-4D14-8D64C4399FDF
^C[2023-11-30T10:47:18Z INFO  supertini] received signal for program 'wait', bye now!
```

To test it in action open two terminal windows, in the first one run `supertini`:

```bash
$ ./target/debug/supertini --binary-path wait
[2023-11-30T10:47:46Z INFO  supertini] running command 'wait --ls'
2023/11/30 11:47:46 2023-11-30 11:47:46.459529029 +0100 CET m=+0.000034858 042876B8-D336-D18F-071D-12319001FC55
2023/11/30 11:47:47 2023-11-30 11:47:47.459676613 +0100 CET m=+1.000182508 042876B8-D336-D18F-071D-12319001FC55
2023/11/30 11:47:48 2023-11-30 11:47:48.459826163 +0100 CET m=+2.000332064 042876B8-D336-D18F-071D-12319001FC55
2023/11/30 11:47:49 2023-11-30 11:47:49.461136958 +0100 CET m=+3.001642854 042876B8-D336-D18F-071D-12319001FC55
2023/11/30 11:47:50 2023-11-30 11:47:50.461214547 +0100 CET m=+4.001720442 042876B8-D336-D18F-071D-12319001FC55
[2023-11-30T10:47:50Z INFO  supertini] file /home/leseb/supertini-rs/wait changed, notifying channel for reload
[2023-11-30T10:47:50Z INFO  supertini] received termination request, killing pid 2675753
[2023-11-30T10:47:50Z INFO  supertini] running command 'wait --ls'
2023/11/30 11:47:50 2023-11-30 11:47:50.468741249 +0100 CET m=+0.000150692 5709C35C-F348-4008-847E-E7E2BA46C5BF
2023/11/30 11:47:51 2023-11-30 11:47:51.470141374 +0100 CET m=+1.001550813 5709C35C-F348-4008-847E-E7E2BA46C5BF
2023/11/30 11:47:52 2023-11-30 11:47:52.47027086 +0100 CET m=+2.001680300 5709C35C-F348-4008-847E-E7E2BA46C5BF
2023/11/30 11:47:53 2023-11-30 11:47:53.471388593 +0100 CET m=+3.002798032 5709C35C-F348-4008-847E-E7E2BA46C5BF
2023/11/30 11:47:54 2023-11-30 11:47:54.472661886 +0100 CET m=+4.004071331 5709C35C-F348-4008-847E-E7E2BA46C5BF
^C[2023-11-30T10:47:55Z INFO  supertini] received signal for program 'wait', bye now!
```

And in the other one:

```bash
rm -f wait && go build src/wait.go
```
