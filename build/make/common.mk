## Print help
.PHONY: help
help:
	@ echo "$$(tput setaf 2)Usage$$(tput sgr0)"
	@ sed -E '/^\.[a-z_]+:/Id' \
		${MAKEFILE_LIST} \
	| sed -n \
		-e '/^## /{h;s/.*//;:d' \
		-e 'H;n;s/^## /\t/;td' \
		-e 's/:.*//;G;s/\n## /\t/;s/\n//g;p;}' \
	| sed -E 's/^(.+)\t/  $(shell tput setaf 2)make$(shell tput sgr0) $(shell tput setaf 6)\1$(shell tput sgr0)\t/' \
	| column -ts $$'\t'
