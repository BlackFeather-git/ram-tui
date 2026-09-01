# Maintainer: Raven <blackfeatheractual@proton.me>
pkgname=ram-tui
pkgver=0.6.1
pkgrel=1
pkgdesc="Lightweight, aesthetic, cross-platform real-time terminal memory monitor with zero dependencies"
arch=('any')
url="https://github.com/BlackFeather-git/ram-tui"
license=('MIT')
depends=('python>=3.6')
provides=('ram')
conflicts=('ram')
source=("$pkgname-$pkgver.tar.gz::https://github.com/BlackFeather-git/ram-tui/archive/refs/tags/v$pkgver.tar.gz")
sha256sums=('SKIP')

check() {
    cd "$pkgname-$pkgver"
    python3 -m unittest discover tests
}

package() {
    cd "$pkgname-$pkgver"
    install -Dm755 ram "$pkgdir/usr/bin/ram"
    install -Dm644 completions/ram.bash "$pkgdir/usr/share/bash-completion/completions/ram"
    install -Dm644 completions/_ram "$pkgdir/usr/share/zsh/site-functions/_ram"
    install -Dm644 completions/ram.fish "$pkgdir/usr/share/fish/vendor_completions.d/ram.fish"
    install -Dm644 LICENSE "$pkgdir/usr/share/licenses/$pkgname/LICENSE"
    install -Dm644 README.md "$pkgdir/usr/share/doc/$pkgname/README.md"
}
