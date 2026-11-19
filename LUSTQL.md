# LustQL: the design

> **Designed, not built.** This records what the project lead chose on 2026-11-19. Nothing here works yet: [LUST.md](LUST.md) section 14 describes the databases Lust has today. Once a part is built, it moves into LUST.md and is removed from this file.

LustQL (`.lql`) is Lust's database language. A `.lql` file describes a database: its tables, how they link, the rules its data follows, and named questions about it. Programs written in Lust (`.lst`) use it. Underneath it is SQLite, so the data is kept safe through crashes, in one file per database.

The aim is everything SQLite can do, in Lust's style, with SQL's own words wherever SQL has one. What you learn in LustQL should carry straight over to SQLite, Postgres and job interviews.

## What the project lead chose

| Question | Choice |
| --- | --- |
| Where it's written | The database's **shape and its named questions** are in a `.lql` file. In `.lst` code, **every read and write is a `query Shop { }` block**, the only way code touches a database, so Lust and LustQL never mix |
| How a query is laid out | **A block, one step per line**, read top to bottom, for reading and writing alike |
| The words in questions | **SQL's own words** (`from`, `where`, `group by`, `select`, `order by`, `limit`), lowercase, checked by Lust like the rest of Lust |
| Removing a row something links to | **Refused, with a clear message**, unless the link says otherwise |
| A table's shape changes | **Safe changes happen by themselves**; ones that could lose data are written out on purpose |
| All-or-nothing changes | `transaction { }` |
| Images, sounds and other files | **Kept beside the database**, in its own folder, linked from the table |
| Servers | **One `.lql` for both:** a file (SQLite) first, a server database (Postgres) later, with the same tables and questions |
| Arrays | **Stay arrays.** Anything stored goes in a table in a `.lql` file; a program copies between the two itself |
| Extra powers | Keys and links, rules on columns, speed marks, word search and saved views |
| Solis | Game data tables, save games, player and online data, and any other data that should be kept |

## A .lql file

```
database Shop in "shop.db" {
    table Customer (int id, string name, string email unique);
    table Item (string name, float price must price >= 0.0, int stock = 0, image picture);
    table Order (int id, Customer who, Item item, int amount must amount > 0);

    question bestCustomers(int top) {
        from Order
        group by who
        select who.name, sum(item.price * amount) as spent
        order by spent desc
        limit top
    }
}

pass Shop;
end
```

- **One database per `.lql` file**, named like an object. `in "shop.db"` says where its file is, found as databases are today (LUST.md section 14).
- A `.lql` file has the same file rules as a `.lst` file: statements end with `;`, the file ends with `end`, and the database is shared with `pass`.
- **A program brings it in** like any other file: `fetch "data/shop":Shop;`.
- A `.lql` file holds no code that runs: only tables, questions and views.

### Tables

`table Name (columns);` makes a table. Its name is also the name of its rows' type, so `Customer` is both the table and one customer.

- **Column types:** `int`, `float`, `string`, `bool`, state options, `vec2`, `vec3`, `rotation`, `colour`, the media types (`image`, `sound`, `file`), and links to other tables.
- **Starting values** follow the column: `int stock = 0`. Old rows get it when a column is added later.
- **Every plain column always has a value.** Lust has no "empty" number or text, so a column that might not be filled in gets a starting value (`string nickname = ""`). Only links can be empty (below).

### Keys

- **The first column is the key** (the *primary key*): every row has a different one, and `Shop.Customer[7]` finds a row by it. An `int` key counts up by itself, and a `string` key is given, as in today's tables.
- **Two columns together** can be the key, written after the columns: `table Seat (int row, int number, string name) key (row, number);`.

### Links

A column whose type is another table links to one of its rows: `Customer who`. In SQL this is a *foreign key*: it holds the other row's key, and the database makes sure that row exists.

- **Follow a link with a dot:** `order.who.name`, in code and in questions.
- **Links are always quick** to follow and to search by. They get an index without asking.
- **An empty link** is allowed only when written with `optional`: `optional User owner`. It's then `nothing` when empty, and code checks it like any `nothing`: `is task.owner !== nothing?`.

**Removing a row that others link to** is refused by default, with a plain message: `can't remove Ada from Customer: 3 rows in Order still link to it.` A link can say otherwise:

| Written | Removing the linked row |
| --- | --- |
| `Customer who` | is refused while rows link to it |
| `Post post remove with it` | removes these rows too |
| `optional User owner empty on remove` | leaves these rows, with the link empty |

### Rules on columns

Broken rules are refused with a plain message, and nothing is changed.

| Rule | Means | SQL's word |
| --- | --- | --- |
| `unique` | no two rows share this value | `UNIQUE` |
| `must condition` | every row has to pass it: `int age must age >= 0` | `CHECK` (Lust's `check` already means going through a state's options) |
| `optional` | a link may be empty | a column that allows `NULL` |
| `unique (a, b)` after the columns | no two rows share both values | `UNIQUE (a, b)` |

### Speed marks

`fast` after a column gives it an index, so questions that search or sort by it stay quick in big tables: `int points fast`. Keys, links and `unique` columns are fast already. A speed mark costs a little on every change, so it's for columns that are searched often.

### Word search

`searchable` after a text column lets questions find words inside it, best match first. It's for long text: notes, dialogue, item descriptions. Underneath, it's SQLite's full-text search.

```
table Note (int id, string title, string body searchable);

question find(string words) {
    from Note
    where body matches words
    order by match desc
}
```

### Media: images, sounds and files

`image`, `sound` and `file` columns keep media. **The files stay beside the database**, in a folder named after it (`shop.db` keeps them in `shop.media/`), and the table holds a link to each.

```
query Shop {
    insert into Item
    set name = "Rain", price = 2.5, stock = 10, picture = Image:file("rain.png")
}
-- Lust copies rain.png into shop.media/ and keeps the link
```

- Big files are fine: they're never loaded until used.
- A file no row links to any more is removed from the folder.
- Moving or copying a database means moving its `.media` folder with it. Lust says so if the folder is missing.
- **In Solis**, a media column links to an asset in the project instead, by its ID, so renaming the file never breaks it.

## Questions

A question is written in SQL's words, lowercase, one step per line, read from top to bottom. Lust checks every name and type in it, so a mistake shows up before the program runs, in plain English. The same steps are used in a named `question` in a `.lql` file and in a `query` block in code (below).

```
question cheapInStock(float under) {
    from Item
    where price < under and stock > 0
    order by price
    limit 10
}
```

- **One step per line**, and no `;` inside: the block's `}` ends it.

| Word | Does | |
| --- | --- | --- |
| `from Table` | where the rows come from | always first |
| `join Table on condition` | adds rows from a table with no link to follow | links are usually followed with a dot instead |
| `where condition` | keeps the rows where it's true | |
| `group by columns` | puts rows with the same values together | |
| `having condition` | keeps the groups where it's true | after `group by` |
| `select columns` | what each answer holds; `as name` names one | left out, each answer is a whole row |
| `distinct` | removes answers that are the same | after `select` |
| `order by columns` | sorts, smallest first; `desc` after a column for biggest first | |
| `limit n`, `offset n` | only n answers, and skip the first n | |

- **Totals** for groups: `count()`, `sum(x)`, `avg(x)`, `min(x)` and `max(x)`.
- **Conditions** use `and`, `or` and `not`, as SQL does, along with Lust's comparisons (`==`, `!=`, `<`). `&&` and `||` are mistakes in a question, and Lust suggests `and` and `or`.
- **A question takes values** like a method: `question bestCustomers(int top)`. They're used anywhere in it.
- **What it gives back** is rows. When `select` is left out, each answer is a whole row of the `from` table. Otherwise each answer has the columns `select` names.

### Saved views

A view is a question that acts like a table, so other questions can use it in `from` and `join`. It takes no values.

```
view RichCustomers {
    from Order
    group by who
    having sum(item.price * amount) > 1000.0
    select who
}
```

## Using it from Lust: query blocks

In a `.lst` file, **a `query` block is the only way to touch a database.** Outside it is Lust; inside it is LustQL. So it's always clear which language a line is in, and a database never changes without a `query` saying so.

```
fetch "data/shop":Shop;

float under = 3.0;

-- reading: the block gives back rows, and the loop around it is Lust
loop (Item it in query Shop {
    from Item
    where price < under and stock > 0
    order by price
    limit 5
}) {
    print(it.name);
}

-- a named question from the .lql file
loop (string name, float spent in query Shop { bestCustomers(5) }) {
    print(name);
}

-- changing rows
query Shop {
    update Item
    set stock = stock - 1
    where name == "tea"
}

-- adding a row (the id counts up by itself)
query Shop {
    insert into Customer
    set name = "Ada", email = "ada@example.com"
}

-- removing rows
query Shop {
    delete from Item
    where stock == 0
}
```

| A block that | Starts with | Then |
| --- | --- | --- |
| reads | `from Table` | the steps in the table above, or a named question on its own: `bestCustomers(5)` |
| changes rows | `update Table` | `set column = value, …`, then `where` for which rows (left out, every row) |
| adds a row | `insert into Table` | `set column = value, …`; a column left out gets its starting value |
| removes rows | `delete from Table` | `where` for which rows. Removing every row needs `where true`, written on purpose |

- **One thing per block:** one read, or one change. Several changes that belong together go in a `transaction` (below).
- **Lust's values are used by name** inside a block: `where price < under`. Lust checks their types, and they're never pasted in as text, so a name a player types can never break the database.
- **A name inside a block is a column first.** If the table has no column by that name, it's a Lust value from around the block (a variable, a field, a method's value). When a Lust value has the same name as a column, the column wins, and Lust warns, so pick a different name for the value.
- **What a reading block gives back** is rows, looped through or put in a variable. Put in a single row's variable (`Item tea = query Shop { from Item where name == "tea" };`), it's the first answer, or `nothing` if there isn't one.
- **A row read into Lust is a copy.** Changing `tea.stock` in code changes the copy, not the database: the database only changes in a `query`.
- **`insert into … set …`** isn't how SQLite writes it (`insert into Item (name, price) values ("Rain", 2.5)`), but it reads the same way as `update`, keeps each value next to its column, and Lust turns it into SQLite's form.

### All or nothing

```
Customer ada = query Shop { from Customer where name == "Ada" };
Item tea = query Shop { from Item where name == "tea" };

transaction {
    query Shop {
        update Item
        set stock = stock - 1
        where name == "tea"
    }
    query Shop {
        insert into Order
        set who = ada, item = tea, amount = 1
    }
}
```

- Everything inside happens fully, or not at all. If something inside fails or the program stops halfway, the database is left as it was before the block. Lust code can go between the queries.
- Transactions can't be put inside each other.

### What changes from today

- **Today's `database { }` blocks in `.lst` files move to `.lql` files,** and today's table methods (`Shop.items:inject`, `Shop.items["tea"].stock = 9`, `:where`, `:orderBy`, `:first`) are replaced by `query` blocks. Once LustQL is built, the old ways are mistakes that show the `query` block to write instead.
- **Arrays stay arrays.** Nothing in a class is ever stored in a database without a `query` writing it there.

## When a table's shape changes

The `.lql` file is the truth. When a program opens the database and the file on disk has an older shape:

- **Safe changes happen by themselves:** a new table, or a new column with a starting value. Old rows get the starting value. So do new rules, as long as every row already passes them.
- **Changes that could lose data** are refused with a message saying how to do them on purpose: removing or renaming a column, changing its type, or a new rule some rows break. They're written in a `change` block, which runs once:

```
change Item {
    rename cost to price;
    remove oldNotes;
}
```

- Each change runs inside a transaction, so a change that fails leaves the database as it was.
- A `change` block that has already run is skipped. Once every copy of the database has it, it can be deleted.

## Where the data lives

- **`in "shop.db"`:** a file on this computer, kept by SQLite. Built first.
- **`at server "…"`, later:** the same database on a server (Postgres), for data many players share at once: accounts, high scores, progress. The tables, rules and questions stay exactly the same; only this line changes. Built as its own step once an online game or app needs it.

## In Solis

Solis uses LustQL as its own database language, for:

- **Game data tables:** items, quests, dialogue lines, loot tables. They're edited in a table panel in the editor, which reads and writes the `.lql` file, and game code fetches them like any `.lql`. The panel is mocked up and approved before it's built.
- **Save games:** big or growing saves (a world's changes, an inventory of thousands), as a database instead of one JSON file. Small saves stay `save`.
- **Player and online data**, through `at server` once it's built.
- **Any other data a game keeps.**

Media columns link to the project's assets by their IDs (above). Solis's runtime is Rust, so it has SQLite either way: under Luau as well as in built games.

## Build order

Each step is one pull request, with its tests, LUST.md section, ROADMAP row and alpha.

1. **`.lql` files and `query` blocks:** databases and tables move out of `.lst` files, with keys (two-column ones too), starting values and `fetch`; `query` blocks with `insert`, `update`, `delete` and a simple `from … where`, replacing today's table methods.
2. **Links:** link columns, following them with dots, `optional`, and what happens on removing.
3. **Rules and speed:** `unique`, `must` and `fast`.
4. **Questions:** `select`, `order by`, `limit`, `offset`, `distinct` and `join`, and named `question`s in `.lql` files.
5. **Groups and views:** `group by`, `having`, the totals, and `view`.
6. **Transactions and shape changes:** `transaction { }`, the safe changes, and `change` blocks.
7. **Word search:** `searchable` and `matches`.
8. **Media:** `image`, `sound` and `file` columns, and the `.media` folder.
9. **Solis:** its side is built in Solis, in its own milestones.
10. **Servers (later):** `at server`, on Postgres.

## Still open

These get settled while building, and each answer is recorded here before it's built:

- How a question's answers are named when looping: the `select` names in order, as in the examples above, or a row type made from them.
- Whether a reading block's rows can be kept in an array variable, and which kind.
- How a view's `table` and `list` show a database's rows (LUST.md section 17, which takes a table today): most likely a `query` block as the rows' source.
- Whether `join` is needed at all once links have dots, or only for tables that don't link.
- Text columns that only hold a few choices: whether a state option is always the better way, so Lust suggests it.
- How `lust run` and LustScript inside other programs (Garry's Mod) reach SQLite, where the host has its own.
