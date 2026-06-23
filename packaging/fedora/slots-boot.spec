%global repo initramfs-slots-boot

Name:           slots-boot
Version:        0.1.0
Release:        1%{?dist}
Summary:        A slot machine that holds your boot hostage until you hit the jackpot

License:        MIT
URL:            https://github.com/Elandig/%{repo}
Source0:        %{url}/archive/refs/tags/v%{version}/%{name}-%{version}.tar.gz

BuildRequires:  cargo
BuildRequires:  rust
Requires:       dracut

%description
slots-boot runs from the initramfs and refuses to let the boot finish until you
spin 7-7-7 on the reels. It's a novelty boot gate; a recovery word (noted in the
README) is the way out if you ever need it.

%prep
# GitHub's archive unpacks to <repo>-<version>/, which differs from Name.
%autosetup -n %{repo}-%{version}

%build
# dracut's inst_binary resolves the binary's libraries, so a normal build is fine.
cargo build --release --locked

%check
cargo test --release --locked

%install
install -Dm755 target/release/slots-boot %{buildroot}%{_bindir}/slots-boot
install -Dm755 initramfs/dracut/90slots/module-setup.sh %{buildroot}%{_prefix}/lib/dracut/modules.d/90slots/module-setup.sh
install -Dm755 initramfs/dracut/90slots/slots-hook.sh %{buildroot}%{_prefix}/lib/dracut/modules.d/90slots/slots-hook.sh
install -Dm644 initramfs/dracut/90slots/slots-boot.service %{buildroot}%{_prefix}/lib/dracut/modules.d/90slots/slots-boot.service
install -Dm644 packaging/dracut/90-slots.conf %{buildroot}%{_sysconfdir}/dracut.conf.d/90-slots.conf

%files
%license LICENSE
%doc README.md
%{_bindir}/slots-boot
%config(noreplace) %{_sysconfdir}/dracut.conf.d/90-slots.conf
%dir %{_prefix}/lib/dracut/modules.d/90slots
%{_prefix}/lib/dracut/modules.d/90slots/module-setup.sh
%{_prefix}/lib/dracut/modules.d/90slots/slots-hook.sh
%{_prefix}/lib/dracut/modules.d/90slots/slots-boot.service

%post
# (Re)build the initramfs so the module lands in it.
if [ $1 -ge 1 ]; then
    dracut --force || :
fi

%postun
# On full removal, rebuild without the module.
if [ $1 -eq 0 ]; then
    dracut --force || :
fi

%changelog
* Tue Jun 23 2026 Elandig <elan@sestudio.org> - 0.1.0-1
- Initial package.
