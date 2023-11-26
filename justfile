
build:
  cargo tauri build || echo "warning: cargo tauri build failed"
  sudo cp target/release/helm /usr/bin/helm

xephyr:
  cargo tauri build || echo "warning: cargo tauri build failed"
  Xephyr -ac -screen 2560x1080 :2 &
  DISPLAY=:2 target/release/helm || echo "warning: helm exited with error"
