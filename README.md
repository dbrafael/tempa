#Tempa - Simple template parsing written in Rust

This is a small tool to help me clone directories replacing template variables in files.
Template variables can have any specified delimiters and the values are supplied in a yaml file.

Example:

Cloning folder "test/" into "out/", replacing any instance of {#variable#} with the appropriate value.

    $ ls
    tempa
    test/
    requirements.yaml # prog:
                      #     name: Tempa
                      #     something: Hello

    $ ls ./test
    file1.txt         # {#prog.something#}, this is {#prog.name#}
    file2.txt         # Hello, this variable doesn't exist {#prog.what#}


    # Will create the directory ./out matching the structure of ./test and replace all variables in the files
    $ tempa -d "{#" -c "#}" -i ./test -o ./out -r ./replacements.yaml
    ...

    $ ls
    tempa
    test/
    out/
    requirements.yaml

    $ ls ./out
    file1.txt         # Hello, this is Tempa
    file2.txt         # Hello, this variable doesn't exist {#prog.what#}

Building:
    git clone https://github.com/dbrafael/tempa
    cd tempa
    cargo build --release
    # Tempa builds to target/release/tempa
