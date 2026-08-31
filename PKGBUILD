# Maintainer: Raven (BlackFeather) <https://github.com/BlackFeather-git>
pkgname=ram-tui
pkgver=0.5.0
pkgrel=1
pkgdesc="Lightweight, fast, cross-platform real-time terminal memory monitor with zero dependencies"
arch=('any')
url="https://github.com/BlackFeather-git/ram-tui"
license=('MIT')
depends=('python>=3.6')
source=("$pkgname-$pkgver.tar.gz::https://github.com/BlackFeather-git/ram-tui/archive/refs/tags/v$pkgver.tar.gz")
sha256sums=('SKIP')

package() {
    cd "$pkgname-$pkgver"
    install -Dm755 ram "$pkgdir/usr/bin/ram"
    install -Dm644 LICENSE "$pkgdir/usr/share/licenses/$pkgname/LICENSE"
    install -Dm644 README.md "$pkgdir/usr/share/doc/$pkgname/README.md"
}
