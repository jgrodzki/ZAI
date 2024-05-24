# Projekt Zaawansowane Aplikacje Internetowe

![Przykładowy widok z aplikacji](demo.png)

## Opis

Aplikacja umożliwiająca dodawanie przedmiotów przez administratora, przeglądanie i ocenianie tych przedmiotów przez użytkowników oraz przeglądanie ocen innych użytkowników.

## Architektura aplikacji

Strona wykonana jako monolityczna aplikacja SPA, backend wykonany w języku Rust, wykorzystana baza danych to PostgreSQL, frontend wykorzystujący SSR z częściowymi aktualizacjami zawartości strony z użyciem biblioteki HTMX, stylowanie z użyciem framework'a tailwindcss.

## Uruchamianie aplikacji

### Wymagania wstępne

* zainstalowane środowisko języka Rust
* zainstalowany i uruchomiony PostgreSQL

### Kroki

Ze względu na wykorzystywane przez aplikację makra weryfikujące podczas kompilacji poprawność użytych zapytań SQL, baza danych musi zostać skonfigurowana przed zbudowaniem aplikacji.

Link do bazy wskazujemy w zmiennej ``DATABASE_URL`` w pliku ``.env``.

Aby ręcznie przeprowadzić migrację, musimy zainstalować narzędzie ``sqlx-cli``:

```sh
cargo install sqlx-cli
```

Narzędzie ``sqlx-cli`` znajduje się w folderze ``~/.cargo/bin``, wieć należy dodać tą ścieżkę do ścieżki systemowej.

Inicjalizujemy bazę będąc w folderze projektu:

```sh
sqlx database reset
```

Aplikację budujemy i uruchamiamy za pomocą narzędzia ``cargo``:

```sh
cargo run --release 
```

Aplikacja jest domyślnie dostępna pod adresem ``localhost:3000``.

W domyślnej migracji bazy danych znajduje się kilka przedmiotów oraz kont wykorzystanych do celów testowych. Dane przykładowe pozyskane ze strony
``myanimelist.net``. Wszystkie konta testowe mają ustawione hasło ``password``.
