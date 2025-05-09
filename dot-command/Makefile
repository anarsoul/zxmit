SOURCES=$(shell find . -type f -iname  "*.asm")
BINARY=zxmit
VERSION=$(shell cat version.def)

ay:
		sjasmplus dot.asm -DAY -DV=$(VERSION)
		
zxuno:
		sjasmplus dot.asm -DUNO -DV=$(VERSION)

next:
		sjasmplus dot.asm -DNEXT -DV=$(VERSION)

clean:
		rm -f $(BINARY) *.sld
