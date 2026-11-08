#!/bin/sh
# Cria o papel da aplicação (sem BYPASSRLS, sem superusuário) com a senha do ambiente.
# As migrations concedem as permissões; este script só existe porque a senha não vai para o repositório.
set -eu
psql -v ON_ERROR_STOP=1 -v senha="$VIDACOR_APP_SENHA" --username postgres --dbname vidacor <<'SQL'
CREATE ROLE vidacor_app LOGIN NOSUPERUSER NOBYPASSRLS NOCREATEDB NOCREATEROLE PASSWORD :'senha';
SQL
