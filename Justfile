
clone-example-repo:
  #!/usr/bin/env bash
  if [ ! -d "example-repo" ]; then
    git clone https://github.com/BurntSushi/ripgrep example-repo
  fi

build: clone-example-repo
  cargo build --release