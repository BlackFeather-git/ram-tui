# Maintainer: Raven <blackfeatheractual@proton.me>
pkgname=ram-tui
pkgver=1.0.3
pkgrel=1
pkgdesc="Blazing-fast, aesthetic, native terminal memory monitor with deep kernel telemetry and zero runtime dependencies"
arch=('x86_64' 'aarch64')
url="https://github.com/BlackFeather-git/ram-tui"
license=('MIT')
makedepends=('cargo')
provides=('ram' 'ram-tui')
conflicts=('ram')
source=("$pkgname-$pkgver.tar.gz::https://github.com/BlackFeather-git/ram-tui/archive/refs/tags/v$pkgver.tar.gz")
sha256sums=('SKIP')

prepare() {
    cd "$pkgname-$pkgver"
    cargo fetch --locked --target "$CARCH-unknown-linux-gnu"
}

build() {
    cd "$pkgname-$pkgver"
    export RUSTUP_TOOLCHAIN=stable
    export CARGO_TARGET_DIR=target
    cargo build --frozen --release --all-targets
}

check() {
    cd "$pkgname-$pkgver"
    export RUSTUP_TOOLCHAIN=stable
    cargo test --frozen --workspace
}

package() {
    cd "$pkgname-$pkgver"
    install -Dm755 target/release/ram "$pkgdir/usr/bin/ram"
    install -Dm755 target/release/ram-tui "$pkgdir/usr/bin/ram-tui"
    install -Dm644 LICENSE "$pkgdir/usr/share/licenses/$pkgname/LICENSE"
    install -Dm644 README.md "$pkgdir/usr/share/doc/$pkgname/README.md"
    install -Dm644 CHANGELOG.md "$pkgdir/usr/share/doc/$pkgname/CHANGELOG.md"
}
