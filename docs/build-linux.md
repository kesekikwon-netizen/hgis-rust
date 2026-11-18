# Linux에서 Rust 워크스페이스 빌드

Windows 절차는 [`build-windows.md`](./build-windows.md)(C++/OSGeo4W)와 [`HANDOVER.md`](./HANDOVER.md) 3절이다. 여기 적은 것은 그 제품 코드를 바꾸지 않고, 이 저장소를 Linux에서 `cargo build` / `cargo test` / `cargo clippy` 할 때 막히던 환경이다.

컴파일러는 `rust-toolchain.toml`의 `1.99.1`. PATH에 `cmake`와 `sqlite3` 명령줄 도구가 있어야 한다. `proj-sys` 0.27.0은 PROJ를 CMake로 소스 빌드하고(`build.rs`의 `cmake::Config::new`), `proj.db`를 만들 때 `sqlite3`를 부른다. `strata-engine`의 주석과 `.github/workflows/ci.yml`이 같은 요구다. GEOS(`geos` 11.3.1, `static`)와 GDAL(`gdal-src` 0.3.0, 앱이 `gdal` 기능을 켠 채로 의존)도 소스 빌드라 첫 빌드는 길다.

## `libstdc++` (GCC 14)

이 이미지에서 `c++`는 clang++이다(`update-alternatives`: manual, 현재 링크 `/usr/bin/clang++`. `g++`는 GCC 13인데 우선순위만 더 높고 선택되어 있지 않다). CMake는 `CXX`가 비어 있으면 `/usr/bin/c++`를 고른다. clang 18은 GCC 설치로 `/usr/lib/gcc/x86_64-linux-gnu/14`를 고르는데, 그 디렉터리의 `libstdc++.so`는 `libstdc++-14-dev`가 놓는다. 런타임 `libstdc++.so.6`과 GCC 13의 `libstdc++-13-dev`만 있으면 링크가 이렇게 죽는다.

```text
/usr/bin/ld: cannot find -lstdc++: No such file or directory
```

PROJ·GEOS·GDAL 소스 빌드가 같은 링커 오류로 바로 끝난다. 코드 문제가 아니다.

```bash
sudo apt-get install -y libstdc++-14-dev
```

확인: `echo 'int main(){return 0;}' | clang++ -x c++ - -o /tmp/linktest` 가 링크된다. `c++`를 `g++-13`으로 돌려 피하는 방법은 쓰지 않는다. clang이 고른 GCC 14와 개발 파일을 맞추는 쪽이 이 이미지의 실제 구성이다.

## ALSA

`rodio` 0.23 → `cpal` 0.18.2 → `alsa-sys` 0.4.0. 그 `build.rs`는 항상 `pkg-config`로 `alsa`를 찾고, 없으면 Debian/Ubuntu에 `libasound2-dev`를 설치하라고 하고 패닉한다. 런타임 `libasound.so.2`만 있으면 `alsa.pc`가 없어 여기서 멈춘다.

```bash
sudo apt-get install -y libasound2-dev
```

## Wayland·X11·xkbcommon

이 잠금 파일에서는 컴파일에 개발 꾸러미가 필요 없다.

- `eframe` 0.31.1 기본 기능이 `winit/default`이고, `winit` 0.30.13 기본 기능에 `wayland-dlopen`이 있다.
- `wayland-sys` 0.31.11 `build.rs`는 `CARGO_FEATURE_DLOPEN`이 있으면 pkg-config를 호출하지 않고 돌아간다. `glutin` 0.32.3도 `wayland-sys`에 `dlopen`을 켠다.
- `x11-dl`과 `xkbcommon-dl` 0.4.2는 실행 중에 `libxkbcommon.so.0` 등을 `dlopen`한다. 창을 실제로 여는 실행에는 런타임 라이브러리가 있어야 하고, `cargo test`가 창을 열지 않으면 개발 꾸러미(`libwayland-dev`, `libxkbcommon-dev`, `libx11-dev`)는 빌드를 막지 않는다.

잠금이 `dlopen`을 끄면 `wayland-sys`가 `wayland-client`(그리고 켜진 기능에 따라 `wayland-cursor`, `wayland-egl`)를 pkg-config로 찾는다. 그때는 `libwayland-dev`가 필요하다.

## 명령

저장소 루트에서. 첫 빌드는 PROJ·GEOS·GDAL 소스 컴파일이 들어 있다.

```bash
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

Windows에서 수치지형·유산의 숨은 브라우저(WebView2)는 그대로다. 그 크레이트는 `[target.'cfg(windows)'.dependencies]`에만 있고, Linux에서는 그 경로가 컴파일되지 않는다.
