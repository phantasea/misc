PREFIX ?= /usr/local
MANDIR ?= $(PREFIX)/share/man
DOCDIR ?= $(PREFIX)/share/doc/fffm

all:
	@echo Run \'make install\' to install fffm.

install:
	@mkdir -p $(DESTDIR)$(PREFIX)/bin
	@mkdir -p $(DESTDIR)$(MANDIR)/man1
	@mkdir -p $(DESTDIR)$(DOCDIR)
	@cp -p fffm $(DESTDIR)$(PREFIX)/bin/fffm
	@cp -p fffm.1 $(DESTDIR)$(MANDIR)/man1
	@cp -p README.md $(DESTDIR)$(DOCDIR)
	@chmod 755 $(DESTDIR)$(PREFIX)/bin/fffm

uninstall:
	@rm -rf $(DESTDIR)$(PREFIX)/bin/fffm
	@rm -rf $(DESTDIR)$(MANDIR)/man1/fffm.1
	@rm -rf $(DESTDIR)$(DOCDIR)
