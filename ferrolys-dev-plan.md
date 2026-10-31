# Ferrolys — план разработки

> **Ferrolys** — открытый лёгкий лаунчер Minecraft на Rust + Slint.
> Linux-first, строгий Material Design 3, без своих серверов и телеметрии, ИИ — по ключам пользователя (BYOK).
>
> Версия плана: 1.0 · 31.10.2026 · темп работы: 5–10 ч/нед

Имя: **ferrum** (железо) + **-lys**. Ржавчина — оксид железа: лаунчер на Rust, который работает с «железом» игры. В софтверном пространстве имя свободно (проверено 31.10.2026: на GitHub и crates.io проектов нет; существует итальянская БАД «Ferrolys» и завод «Ferroloy» — другие товарные категории, конфликта для OSS-проекта нет).

---

## 1. Позиционирование и принципы

**Что такое Ferrolys:** нативный, мгновенно запускающийся лаунчер с честным MD3-интерфейсом, который ставит ваниллу и лоадеры, работает с аккаунтами Ely.by и оффлайн-профилями, а позже добавляет каталог Modrinth, ИИ-сборщик модпаков и рантайм Horizon.

**Принципы (не нарушаются ни при каких обстоятельствах):**

1. **Ноль своих серверов.** Всё, что нужно, качается напрямую: Mojang piston-meta, метаданные лоадеров, Modrinth, GitHub Releases. Нет аккаунтовой системы Ferrolys — значит, нечего терять и некому продавать данные.
2. **Ноль телеметрии.** Никаких «аналитических» запросов из приложения. Сеть видит только то, что пользователь сам попросил скачать.
3. **Приватность по умолчанию.** Токены Ely.by уходят только на authserver.ely.by. Ключи ИИ (BYOK) хранятся локально и уходят только в сконфигурированный пользователем endpoint.
4. **Лёгкость.** Один бинарник, RAM в десятках МБ, старт за доли секунды, никакого webview и JVM внутри лаунчера.
5. **Открытость.** Apache-2.0, публичный репозиторий, релизы на GitHub Releases.

**Аудитория:** игроки на Linux (первая платформа), RU/EN интерфейс с первого дня.

---

## 2. Зафиксированные решения

| Вопрос | Решение |
|---|---|
| Стек | Rust + Slint (femtovg/Skia рендер, софтверный фолбэк) |
| Платформы | **Linux первым** (X11 + Wayland), Windows — волной позже (код портируемый с дня 1) |
| Проект | Публичный open-source, GitHub |
| Имя | **Ferrolys** |
| Аккаунты | **Ely.by + оффлайн-профили**; Microsoft — третья/четвёртая волна (нет лицензионного аккаунта для тестов — дорого) |
| Скины | Ely.by (в игре через authlib-injector) + локальный `skin.png` для оффлайн-профилей |
| MVP-скоуп | Только игра + лоадеры (ванилла, Java, Fabric/NeoForge/Forge/Quilt, запуск). Моды Modrinth, ИИ, Horizon — следующих волн |
| ИИ | Только универсальный OpenAI-совместимый backend (базовый URL + ключ + модель). Покрывает OpenAI, Groq, OpenRouter, vLLM, Ollama |
| Дизайн | Строгий Material Design 3 |
| Тема | Тёмная база + несколько акцентных цветов на выбор |
| Языки UI | RU + EN сразу |
| Horizon | Вторая волна (после стабилизации MVP) |
| Обновления лаунчера | Баннер «вышла версия» + ссылка на GitHub Releases, без авто-обновления |
| Опыт в Rust | Средний → план с плавной кривой входа |
| Лицензия | Apache-2.0 |
| Темп | 5–10 ч/нед |

## 3. Не-цели (осознанно НЕ делаем)

- Свои аккаунты, чаты, друзья, хостинг, экономика — ничего серверного.
- CurseForge — до волны 4 (их API-ключ и политика; Modrinth закрывает 90% потребности).
- Автообновление с перезапуском и подписями — до волны 4.
- macOS — не в этом плане (Slint её поддерживает; вернуться, если появится спрос).
- Встроенный браузер/webview — никогда (принцип лёгкости).
- Редактор сборок на стороне сервера — ИИ работает локально, ключи пользователя.

---

## 4. Архитектура

### 4.1 Workspace

```
ferrolys/
├── Cargo.toml                # workspace
├── crates/
│   ├── ferrolys/             # бинарник: Slint UI, bootstrap, async-мост
│   │   ├── ui/               # .slint файлы (экраны, компоненты)
│   │   │   ├── md3/          # дизайн-кит (см. §5)
│   │   │   └── screens/      # экраны приложения
│   │   └── src/
│   │       ├── main.rs
│   │       ├── bridge.rs     # tokio ⇄ Slint (Weak handle, каналы)
│   │       └── state.rs      # глобальные singletons для .slint
│   ├── ferrolys-core/        # библиотека домена: тесты без UI
│   │   └── src/
│   │       ├── profile/      # инстансы, настройки, пути XDG
│   │       ├── install/      # ванилла, лоадеры, Java, движок загрузок
│   │       ├── launch/       # сборка команды JVM, запуск, логи, краш-детект
│   │       ├── accounts/     # ely, offline, keyring
│   │       └── skins/        # ely-скины, локальный skin.png, превью
│   └── ferrolys-ai/          # волна 3: OpenAI-совм. клиент + пайплайны
├── i18n/                     # ru.ftl, en.ftl (или csv для rust-i18n)
├── assets/fonts/             # Inter (или Roboto) лицензированный
└── .github/workflows/        # CI: fmt, clippy, test, build, release
```

**Правила зависимостей:** `ferrolys-core` не знает про Slint (только types + thiserror). UI вызывает core через async-мост. Так вся логика тестируется юнит-тестами без компиляции UI.

### 4.2 Async-мост Slint ⇄ tokio

- Один глобальный `tokio::runtime` в main.
- Задача из UI → `slint::Weak` + `spawn` → по завершении `upgrade_in_event_loop()` и запись в property / вызов callback.
- Прогресс загрузок: `tokio::sync::watch` → периодический тик (например, каждые 100 мс) обновляет прогресс-модель. Не дёргать UI на каждый чанк.
- Ошибки: core возвращает типизированные ошибки; UI маппит их в человеческие строки через i18n.

### 4.3 Хранение данных (XDG)

| Что | Где |
|---|---|
| Инстансы (игровые папки) | `~/.local/share/ferrolys/instances/<id>/` |
| Общие assets | `~/.local/share/ferrolys/assets/` (одни на все инстансы — они не зависят от лоадера) |
| Java-рантаймы | `~/.local/share/ferrolys/runtimes/<id>/` |
| Настройки | `~/.config/ferrolys/settings.toml` |
| Токены | keyring (Secret Service / KWallet) через крейт `keyring`; фолбэк — файл `0600` с предупреждением в UI |
| HTTP-кэш (иконки, метаданные) | `~/.cache/ferrolys/` |
| Логи лаунчера | `~/.local/state/ferrolys/log/` (tracing, ротация) |

Инстанс = классическая структура `.minecraft`: `versions/`, `libraries/`, `mods/` (волна 2), `logs/`, `options.txt`, `saves/`. Это даёт бесплатную совместимость с гайдами и ручным переносом.

`profile.json` инстанса: имя, иконка, версия игры, `{loader: fabric@0.16.x}`, Java (исполняемый файл, память, доп. аргументы), привязанный аккаунт, параметры скина, `last_played`, `created`.

---

## 5. Дизайн-система: строгий MD3 на Slint

### 5.1 Токены

Глобальный singleton в `.slint`:

- **Цветовые роли:** primary / on-primary / primary-container / on-primary-container; secondary(+container); tertiary(+container); error(+container); surface, surface-container-lowest…highest, on-surface, on-surface-variant; outline, outline-variant; inverse-surface/on-inverse; scrim.
- **Типографика (шкала MD3):** display L/M/S, headline L/M/S, title L/M/S, body L/M/S, label L/M/S — 15 стилей, каждый с размером/межстрочным/весом/трекингом.
- **Формы:** corner-extra-small 4dp … corner-extra-large 28dp + special (full для кнопок/чипов).
- **Elevation:** уровни 0–5, тени (для тёмной темы — на тон поверх surface-container-tonal).
- **State layers:** hover 8% / focus 12% / pressed 10% / dragged 16% (поверх контентного цвета) — обязательны для строгого MD3.
- **Анимации:** стандартные кривые MD3 (emphasized, standard), длительности 50–500 мс. Рябь (ripple) — опциональная волна полировки, не блокирует релиз.

### 5.2 Темы: тёмная база + акценты

- База — тёмная схема.
- Пользователь выбирает **акцентный цвет** из набора пресетов (+ позже произвольный пикер).
- Тональные палитры считаются из seed-цвета портом **material-color-utilities на Rust** (на crates.io есть порт — найти по запросу material-colors; проверить качество до внедрения, фолбэк — предвычисленные палитры для 8 пресетов, захардкоженные из официального инструмента Google).
- Смена акцента — на лету, перерисовка токенов без перезапуска.
- Светлая тема — не делаем (решение зафиксировано); архитектура токенов позволит добавить позже без переписывания.

### 5.3 Инвентарь компонентов (в порядке необходимости)

**Набор MVP (делаем в волнах 0–1):**

| Компонент | Варианты/состояния |
|---|---|
| Button | filled / tonal / outlined / text + disabled + state layers |
| IconButton | standard / filled / tonal / outlined |
| TextField | outlined (label-нотч упрощаем), + filled; валидация, суффикс (показать пароль) |
| Card | filled / outlined (elevated — позже) |
| ListItem | one-line / two-line, leading/trailing, divider |
| LinearProgress | determinate / indeterminate |
| CircularProgress | determinate / indeterminate |
| Switch | on/off/focus |
| Checkbox / Radio | checked/unchecked/indeterminate(только checkbox) |
| Dialog | basic + scrim, заголовок/контент/действия |
| Snackbar | simple + action, авторазмещение снизу |
| NavigationRail | + пункт настроек внизу, selected-индикатор |
| Tooltip | plain |
| Slider | continuous + discrete (настройки памяти) |
| Menu / Dropdown | для выбора версии, лоадера |
| Chips | assist (выбор версии) |
| Scrollbar | тонкая, появляется при скролле |

**Расширенный набор (волны 2–3, по мере экранов):** FAB, FilterChip/InputChip (теги ИИ-сборщика), Tabs, SegmentedButton, Badge (счётчик обновлений модов), SearchBar, BottomSheet, FullScreenDialog, всплывающие карточки описания модов (rich-text упрощённый).

### 5.4 Шрифт

Inter (или Roboto) — переменный, встроен в бинарник через `slint::include_fonts!`. Кириллица обязательна в выбранном шрифте (у Inter и Roboto есть).

### 5.5 Экраны MVP

1. **Onboarding** (первый запуск): выбор языка → расположение данных → проверка Java → «готово».
2. **Профили** (главный): NavigationRail + список карточек-инстансов, FAB «создать».
3. **Профиль (детали)**: версия/лоадер, кнопка Play, память, аргументы JVM, папка, логи последнего запуска, скин.
4. **Мастер создания инстанса**: версия → лоадер → имя → установка с прогрессом.
5. **Аккаунты**: вход Ely.by (логин/пароль, 2FA), создание оффлайн-профиля.
6. **Настройки**: внешний вид (акцент), язык, сеть (потоки загрузки), пути, «о программе» + проверка обновлений.
7. **Лог запуска**: живой поток stdout/stderr, фильтр, кнопка «открыть папку», детект краша.

---

## 6. Волна 0 — Каркас (недели 1–3)

Цель: скелет, на котором видно всё: CI, i18n, токены, навигация, async-мост.

- [ ] `cargo new` workspace (2 крейта), rust-toolchain.toml (stable), rustfmt/clippy конфиги
- [ ] Slint-скелет: окно 480×720…1280×800, NavigationRail + пустые страницами
- [ ] Мост tokio ⇄ Slint: демо-задача с прогрессом (скачивание тестового файла)
- [ ] Модуль i18n: словарь в Rust (rust-i18n или fluent-bundle), строки подаются в `.slint` через глобальный singleton; RU/EN, переключение на лету. Все строки — только через словарь с первого дня
- [ ] Дизайн-токены: цвета (тёмная схема, 2 пресета акцентов), типографика, формы, state layers
- [ ] Компоненты: Button (4 варианта), TextField, Card, ListItem, Switch, LinearProgress
- [ ] Настройки: `settings.toml` + экран настроек (язык, акцент) — рабочие
- [ ] XDG-пути (`dirs`), `tracing` + ротация логов, panic hook → диалог
- [ ] CI: GitHub Actions (ubuntu) — fmt, clippy -D warnings, test, release build; артефакт tar.gz
- [ ] README (RU/EN), LICENSE Apache-2.0 + NOTICE (Slint), CONTRIBUTING минимальный

**DoD волны 0:** запуск `cargo run` открывает окно с MD3-навигацией, переключение RU/EN и акцента на лету, CI зелёный.

---

## 7. Волна 1 — MVP: ванилла + лоадеры + аккаунты (недели 4–16)

### 7.1 Ядро установки ваниллы (нед 4–6)

- [ ] Клиент piston-meta: `version_manifest_v2.json` → список версий (release/snapshot/old_beta), кэш с TTL
- [ ] Парсер version-JSON: downloads, assetIndex, libraries (правила `rules` для os/name — на Linux почти всё разрешено, но парсер полный), mainClass, `arguments.game/jvm`, logging, javaVersion.majorVersion
- [ ] Движок загрузок: reqwest + tokio, 4–8 потоков, докачка Range, ретраи с бэкоффом, SHA-1 каждого файла (assets и libraries дают хеши), `.part` → rename; общий прогресс (байты/скорость/ETA)
- [ ] Assets: индекс → `objects/<hh>/<hash>` из `resources.download.minecraft.net`; виртуальный/ресурс-маппинг не нужен для запуска (только `assets/` структура)
- [ ] Libraries: вычисление путей из maven-координат, артефакт + classifiers (`natives-linux`) → распаковка в `versions/<id>/natives` с исключением `META-INF`
- [ ] Учёт `logging.client`: подмена log4j-конфига (фикс Log4Shell-класса уязвимых старых версий)
- [ ] Тесты: fixture-манифесты 1.7.10 / 1.12.2 / 1.20.x / 1.21.x — правила библиотек и полнота загрузок на мок-HTTP (wiremock)

### 7.2 Запуск (нед 7–8)

- [ ] Сборка команды: подстановка шаблонов `${auth_player_name} ${auth_uuid} ${auth_access_token} ${user_type} ${game_directory} ${assets_root} ${assets_index_name} ${version_name} ${classpath} ${natives_directory} ${launcher_name} ${launcher_version}`; JVM-часть из манифеста + `-Xmx` из профиля + `-Djava.library.path`
- [ ] Класспатх-разделитель `:` (Linux), эскейпинг путей с пробелами
- [ ] Спавн процесса (std/tokio::process), стриминг stdout/stderr в live-лог (Slint ListView, батчинг), код выхода
- [ ] Детект краша: паттерны в логе (`Game crashed!`, `Exception in thread "main"`), кнопки «открыть лог»/«открыть папку»
- [ ] Golden-тесты генератора команды для фикстур версий

### 7.3 Java (нед 9)

- [ ] Детект системной Java (`java` в PATH, `JAVA_HOME`); парсинг версии, сравнение с требуемой манифестом
- [ ] Если нет/не подходит — загрузка Temurin (Adoptium API): `api.adoptium.net/v3/binary/latest/<major>/ga/linux/x64/jdk/hotspot/normal/eclipse` (и aarch64), распаковка в `runtimes/`, кэш
- [ ] Выбор Java per-инстанс (переопределение пути)
- [ ] Fallback: предупредить и предложить установку системного пакета (ссылка на доку, не команду sudo)

### 7.4 Аккаунты: Ely.by + оффлайн (нед 10–12)

Ely.by — Yggdrasil-совместимый authserver:

- [ ] Клиент `authserver.ely.by`: `POST /authenticate` (agent, username, password, clientToken) → `accessToken`, `availableProfiles`, `selectedProfile`; `POST /refresh`; `POST /validate`; `POST /invalidate`
- [ ] 2FA (TOTP): обработка требования кода и повторный запрос (точное поле сверить с документацией Ely.by)
- [ ] Хранение: токен + clientToken в keyring; авторефреш перед запуском; «сессия истекла» → экран ввода пароля
- [ ] **authlib-injector**: загрузка jar с GitHub Releases (yushijinhun/authlib-injector), кэш, `-javaagent:<jar>=https://authserver.ely.by` в JVM-аргументы → в игре работают скины Ely и вход на серверы с поддержкой Ely (join в sessionserver.ely.by выполняет игра через перехваченный authlib)
- [ ] Оффлайн-профили: ник → UUID v3 (MD5 от `OfflinePlayer:<ник>` — поведение ванильного клиента), выбор без пароля, метка «только offline-серверы»
- [ ] UI: список аккаунтов, активный на инстанс, вход/выход, смена профиля (если у аккаунта их несколько)
- [ ] Тесты: fixture-ответы authenticate/refresh (все ветки, включая 2FA и неверный пароль); валидация UUID

### 7.5 Лоадеры (нед 12–15)

Порядок — от простого к сложному:

- [ ] **Fabric**: `meta.fabricmc.net/v2/versions/loader/<game>` → выбор loader-версии; профиль `meta.fabricmc.net/v2/versions/loader/<game>/<loader>/profile/json` — готовый version-JSON (библиотеки с maven.fabricmc.net, mainClass Knot). Подмешивается к ванильному манифесту
- [ ] **Quilt**: `meta.quiltmc.org/v3/.../profile/json` — аналогично Fabric
- [ ] **NeoForge**: `maven.neoforged.net/api/maven/versions/releases/net/neoforged/neoforge?filter=<game>` → список; installer-JSON с maven.neoforged.net → библиотеки + mainClass
- [ ] **Механизм processors** (общий для NeoForge/Forge сложных версий): запуск Java-шагов установки (jarsplitter/JarJar/installertools) с скачанными библиотеками инсталлера. Сначала — минимальная реализация под фактические данные NeoForge
- [ ] **Forge** (самый сложный, риск): promotions_slim.json → список; installer-JSON с maven.minecraftforge.net → библиотеки + processors (binpatch/merge). Ожидаемо съест больше всего времени
- [ ] UI: в мастере создания — выбор лоадера с версией, «без лоадера» = ванилла
- [ ] Тесты: парсеры метаданных всех лоадеров на fixture'ах; сравнение резолва библиотек с эталонными списками

### 7.6 Скины (нед 15, по ходу)

- [ ] Локальный `skin.png`: выбор файла, валидация 64×64 (растёт кратно), 2D-превью в карточке профиля, копирование в инстанс
- [ ] Скин аккаунта Ely.by: получение текстуры (эндпоинт по документации Ely.by), кэш, то же превью
- [ ] Честная заметка в UI: скин Ely виден на серверах с поддержкой Ely; оффлайн-скин на сервере — только с серверным плагином типа SkinsRestorer

### 7.7 Полировка и релиз v0.1.0 (нед 16)

- [ ] Баннер обновления: GET GitHub API `releases/latest` (etag-кэш), сравнение semver, баннер + `xdg-open`
- [ ] Обработка «нет сети» / частичных загрузок (resume, `--verify` режим перепроверки хешей)
- [ ] Иконка приложения, .desktop-файл; поставка: tar.gz (+ опционально AppImage)
- [ ] Скриншоты для README (RU/EN), раздел FAQ (Ely, оффлайн, Java, «куда ставится»)
- [ ] Тег `v0.1.0`, черновик релиза, ручной чек-лист ниже

**DoD MVP (проверяется на чистой системе):**

- [ ] Скачан tar.gz → запускается двойным кликом из файлового менеджера, окно < 1 сек
- [ ] Вход в Ely.by (включая аккаунт с 2FA) и создание оффлайн-профиля
- [ ] Установка ваниллы 1.21.x и 1.12.2 (проверка старых манифестов) в два отдельных инстанса
- [ ] Установка Fabric + NeoForge + Quilt на 1.21.x; Forge хотя бы на одной версии
- [ ] Запуск с системной Java и с авто-скачанной Temurin
- [ ] Вход на сервер с поддержкой Ely.by со скином; оффлайн-профиль — на offline-сервер
- [ ] Два инстанса запускаются одновременно, данные не пересекаются
- [ ] RU ⇄ EN на лету; смена акцента на лету
- [ ] RAM лаунчера в простое < 100 МБ; бинарник < 25 МБ
- [ ] Краш игры детектится, лог открывается; лог лаунчера ротируется
- [ ] CI собирает релиз и прикладывает sha256

---

## 8. Волна 2 — Моды Modrinth + Horizon (недели 17–25)

### 8.1 Modrinth

- [ ] Клиент `api.modrinth.com/v2`: search (facets: `versions`, `categories`, `project_type`), project, versions, files (url, sha1, зависимости required/optional)
- [ ] Экран каталога: поиск, фильтры (категория, версия, лоадер), пагинация, иконки через http-кэш
- [ ] Установка мода в инстанс: выбор совместимой версии, резолв required-зависимостей рекурсивно, конфликт-диалог при дубле
- [ ] Вкл/выкл мода (rename `.disabled`), удаление, «обновить все» (проверка новых версий)
- [ ] Импорт/экспорт `.mrpack` (zip + `modrinth.index.json`) — вход и выход
- [ ] Детект «мод не для этой версии/лоадера» до скачивания, человеческое объяснение
- [ ] Тесты: резолвер зависимостей (граф, циклы, конфликты), парсер .mrpack

### 8.2 Horizon (Rust-порт Minecraft)

- [ ] Экран Horizon в NavigationRail: версии (GitHub Releases их репо, linux x86-64), changelog, установка/обновление
- [ ] Prepare-шаг: сборка ресурсов из легальной копии игрока (Minecraft JE 26.3) — подсмотреть механику в их MIT-лаунчере, реализовать у себя
- [ ] Запуск клиента и «host a server»
- [ ] Проверка наличия копии игры (по установленному инстансу/пути), внятная ошибка-подсказка
- [ ] Никакой дистрибуции ассетов Mojang (их EULA) — только оркестрация

**DoD волны 2:** поиск «create» → установка мода с зависимостями в Fabric-инстанс → запуск; импорт чужого .mrpack; Horizon-клиент запускается из Ferrolys.

---

## 9. Волна 3 — ИИ (BYOK) (недели 26–32)

Единый модуль `ferrolys-ai`: клиент OpenAI-совместимого API (`{base_url}/chat/completions`, Bearer-ключ, `response_format: json_object` где поддерживается) + пайплайны. Ключи в keyring, поле модели, кнопка «проверить соединение». Никаких встроенных пресетов — пользователь вводит base URL, ключ и модель сам.

### 9.1 ИИ-сборщик модпаков

Чат «опиши сборку» на экране каталога. Пайплайн (главный принцип: **LLM ничего не изобретает — только выбирает из реальных кандидатов Modrinth**):

1. LLM-вызов 1: из промпта → JSON `{concepts: ["магия","технологии",...], size: "лёгкая|средняя|большая", notes}`.
2. Для каждого концепта — Modrinth search с facets (`versions:<mc>`, `categories:<loader>`) → топ-8 кандидатов (slug, title, описание, downloads, follows).
3. LLM-вызов 2: выбор финального сета из кандидатов + `why`-строка для каждого (JSON-схема в промпте).
4. **Валидатор кодом** (не LLM): совместимость версии/лоадера по данным Modrinth, дубликаты, резолв зависимостей, карантин несовместимых с причиной («не для этой версии», «конфликт зависимости», «дубль»).
5. План → экран-ревью (карточки модов с why, чипы «убрать/добавить») → установка в один клик через движок волны 2.

- [ ] Кэш планов по (промпт, mc-версия, лоадер) — повторный запрос не тратит токены
- [ ] Обработка отказов: 401 (плохой ключ), 429, таймаут, невалидный JSON ответа (расслабленный парсер + повтор)
- [ ] Чипы размера сборки, учёт «лёгкая/большая» в промпте

### 9.2 Crash-ИИ

- [ ] Кнопка «разобрать краш» на экране краша (после волны 1)
- [ ] Скраб лога перед отправкой: `--accessToken ***`, домашние пути, email (по образцу из исследования: это ДО отправки, локально)
- [ ] Хвост 6000 символов + включённые моды → LLM → `{причина, шаги, disable: [файлы]}`
- [ ] Предложения «выключить мод» валидируются списком фактически включённых модов

**DoD волны 3:** сборка «магия+технологии, средняя, 1.21 fabric» собирается и запускается; краш-лог известного мода разбирается с предложением его выключить.

---

## 10. Волна 4 — Windows, Microsoft, CurseForge, авто-апдейт (недели 33+)

- [ ] **Windows**: CI-джоба windows-latest, инсталлер (MSI/WiX или NSIS), тесты на Win10/11: пути, natives-windows, разделитель класспатха `;`, кодировка консоли
- [ ] **Microsoft** (device flow, за флагом `--features msft`): `login.microsoftonline.com/consumers/oauth2/v2.0/devicecode` → поллинг token → XBL → XSTS → Minecraft services → профиль. Автор не может тестировать (нет лицензии) → fixtures на каждый шаг, ручной прогон добровольцами из issues, честная пометка «экспериментально» в UI
- [ ] **CurseForge**: заявка на API-ключ проекту, их SLA/политика; каталог рядом с Modrinth
- [ ] **Авто-обновление**: скачивание + минимальная подпись (minisign), перезапуск; до тех пор — баннер
- [ ] Flatpak (опционально), автоопределение Wayland/X11 остаётся прозрачным

---

## 11. Стек (крейты)

| Назначение | Крейт | Примечание |
|---|---|---|
| UI | `slint` | рендер femtovg + feature sw-фолбэк; live-preview в VS Code |
| Async | `tokio` + `reqwest` (rustls) | один runtime, hyper не напрямую |
| Сериализация | `serde`, `serde_json`, `toml` | манифесты, настройки |
| Хеши | `sha1`, `sha2`, `md-5` | SHA-1 для Mojang/Modrinth, MD5 для offline-UUID |
| Пути | `dirs` | XDG |
| Секреты | `keyring` | Secret Service/KWallet + файловый фолбэк 0600 |
| Ошибки | `thiserror` (core), `anyhow` (main) | |
| Логи | `tracing` + `tracing-subscriber` | ротация, уровень из настроек |
| Версии | `semver` | сравнение версий игры/лоадеров |
| Архивы | `zip` | natives, .mrpack (волна 2) |
| i18n | `rust-i18n` или `fluent-bundle` | один механизм для UI и ошибок |
| Диалоги | `rfd` | выбор skin.png, папок |
| Тесты HTTP | `wiremock` | манифесты, ely, modrinth |
| MD3-палитры | порт material-color-utilities (проверить `material-colors`) | или предвычисленные палитры |
| ИИ (в3) | reqwest + serde | без SDK, обычный JSON |

## 12. Тестирование и CI

- **Юнит:** парсеры манифестов (fixtures 1.7.10→1.21.x), правила libraries, генератор команды запуска (golden-файлы), резолвер зависимостей Modrinth, offline-UUID, скраб краш-лога.
- **Интеграционные:** установка версии на мок-HTTP (wiremock) → проверка файлового дерева и хешей; ely-клиент на fixture'ах.
- **Ручной чек-лист:** DoD каждой волны, прогон на чистой VM Ubuntu + Arch (Wayland и X11), VM без GPU (софтверный рендер).
- **CI:** fmt → clippy -D warnings → test → release build (ubuntu); артефакт на каждый пуш в main; тег → draft-release с tar.gz + sha256; позже windows-джоба.
- Перед релизами: `cargo audit` + `cargo deny` (лицензии зависимостей).

## 13. Риски и меры

| Риск | Вер-ть | Мера |
|---|---|---|
| **Лицензия Slint**: не GPL и не Apache; для бесплатного десктопа — royalty-free license с NOTICE | certainty | Прочитать текст до релиза v0.1.0, положить NOTICE в дистрибутив. Наш код остаётся Apache-2.0 как потребитель |
| **Forge-инсталлер** (processors, binpatch) сложнее остальных лоадеров | высокая | Делать последним; допустим перенос в волну 2 без ущерба MVP (Fabric/Quilt/NeoForge уже закрывают основное) |
| Изменения API Ely.by | средняя | Изоляция в одном модуле + fixture-тесты; сообщения об ошибках без «мусорных» JSON |
| Microsoft-флоу нечем тестировать | высокая | Фича-флаг, fixtures, «экспериментально», тест добровольцами |
| Строгий MD3 = разрастание UI-кита | высокая | Компоненты делаются только под конкретные экраны; расширенный набор — волнами 2–3 |
| Wayland/GL-окружения разнообразны | средняя | Софтверный рендер как фолбэк, тесты в VM; сборка с femtovg по умолчанию |
| Один разработчик, 5–10 ч/нед | certainty | Волны зафиксированы; всё новое — в backlog; релиз малыми версиями |
| Scope creep от идей | высокая | Правило: идея → backlog.md, не в текущую волну |

## 14. Юридические заметки

1. **Apache-2.0** для кода Ferrolys; файлы заголовков не обязательны, но NOTICE для Slint — да.
2. **Не переносить код Millida** (GPL-3.0 — «заразная»): архитектурные идеи можно, строки кода нельзя. Horizon — MIT, их код читать и цитировать с атрибуцией можно.
3. **Mojang EULA/торговые марки**: не распространять ассеты игры (качаем с их серверов по запросу игрока), не использовать официальный арт/название в брендинге Ferrolys, дисклеймер «not affiliated with Mojang» в README и About.
4. authlib-injector — внешний артефакт: скачиваем и запускаем как отдельный инструмент, не встраиваем код (его лицензию проверить при встраивании, нам встраивание не нужно).
5. Шрифты — только с разрешающей лицензией (Inter — OFL).

## 15. Вехи (сводка)

| Волна | Срок (от старта) | Релиз | Содержание |
|---|---|---|---|
| 0 Каркас | 3 нед | — | Скелет, MD3-токены, i18n, CI |
| 1 MVP | ~16 нед | **v0.1.0** | Ванилла + лоадеры + Ely/оффлайн + скины + запуск |
| 2 Моды + Horizon | ~25 нед | v0.2.0 | Modrinth, .mrpack, Horizon |
| 3 ИИ | ~32 нед | v0.3.0 | BYOK-сборщик, crash-ИИ |
| 4 Экспансия | 33+ нед | v0.4.0 | Windows, MS-аккаунты (эксп.), CurseForge, авто-апдейт |

При старте в начале ноября 2026 и темпе 5–10 ч/нед: **MVP — конец февраля 2027**, моды+Horizon — конец апреля, ИИ — конец июня. Оценки волоконные: каждая следующая волна стартует только по DoD предыдущей.

## 16. Backlog (идеи вне волн — не сейчас)

- Пинг серверов и список серверов в лаунчере (статус, онлайн, иконка)
- Импорт существующего `.minecraft` и профилей Prism/HMCL
- Бэкап/снапшоты инстансов, экспорт в архив
- Менеджер скриншотов инстанса
- Светлая тема (токены уже позволят)
- Расширенная диагностика Java (проверка GL-версии, lwjgl-логи)
- Ollama-детект на localhost с автонастройкой
- Профили JVM-пресетов («оптимизация», «моды»)
- Оверлей в игре (долгосрочно, отдельный проект)

## 17. Ссылки (справка разработчика)

- Slint: https://slint.dev · книга https://docs.slint.dev
- Манифесты Mojang: https://piston-meta.mojang.com/mc/game/version_manifest_v2.json · ассеты https://resources.download.minecraft.net/
- Fabric meta: https://meta.fabricmc.net/v2/versions/loader
- Quilt meta: https://meta.quiltmc.org/v3/versions/loader
- NeoForge maven: https://maven.neoforged.net/api/maven/versions/releases/net/neoforged/neoforge
- Forge promotions: https://files.minecraftforge.net/net/minecraftforge/forge/promotions_slim.json
- Ely.by authserver: https://authserver.ely.by (документация на сайте Ely.by)
- authlib-injector: https://github.com/yushijinhun/authlib-injector
- Adoptium API: https://api.adoptium.net
- Modrinth API: https://api.modrinth.com/v2 · https://docs.modrinth.com
- Horizon: https://playhorizon.dev (MIT, репозиторий на GitHub)
- MD3: https://m3.material.io · порты material-color-utilities на Rust — искать на crates.io
- Microsoft device flow: https://login.microsoftonline.com/consumers/oauth2/v2.0/devicecode (волна 4)

---

*План живой: волны не меняются местами, содержимое внутри волны можно двигать. Любая новая идея сначала в backlog (§16), потом в волну.*
