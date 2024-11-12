
CC=gcc

# Nom de l'exécutable
EXEC = ccrun

SRC = main.c

# Fichiers objets
OBJ = $(SRC:.c=.o)

CFLAGS = -Wall -Wextra -g

all: $(EXEC)

$(EXEC): $(OBJ)
	$(CC) $(OBJ) -o $(EXEC)

# Règle pour compiler les fichiers .c en fichiers .o
%.o: %.c
	$(CC) $(CFLAGS) -c $< -o $@

clean:
	rm -f $(OBJ) $(EXEC)

# Indiquer que les règles ne sont pas des fichiers
.PHONY: all clean