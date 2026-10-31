use desktop_design_system::is_system_dark_mode;
use rusqlite::{params, Connection};
use slint::VecModel;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};

// Include the standalone isolated Slint UI compiled by build.rs
include!(concat!(env!("OUT_DIR"), "/pos_terminal.rs"));

static ORDER_COUNTER: AtomicUsize = AtomicUsize::new(1042);

#[derive(Clone, Debug)]
struct DbProduct {
    id: i32,
    sku: String,
    name: String,
    category: String,
    price_cents: i32,
    stock_qty: i32,
}

#[derive(Clone, Debug)]
struct ActiveCartItem {
    product_id: i32,
    sku: String,
    name: String,
    unit_price_cents: i32,
    qty: i32,
    discount_pct: i32,
}

struct PosTerminalState {
    products: Vec<DbProduct>,
    cart: Vec<ActiveCartItem>,
    current_category: usize, // 0: All, 1: Beverages, 2: Bakery, 3: Hardware, 4: Merch
    search_query: String,
    tendered_cents: i32,
    order_no: String,
}

impl PosTerminalState {
    fn new(products: Vec<DbProduct>) -> Self {
        let order_id = ORDER_COUNTER.fetch_add(1, Ordering::SeqCst);
        Self {
            products,
            cart: Vec::new(),
            current_category: 0,
            search_query: String::new(),
            tendered_cents: 0,
            order_no: format!("ORD-{}", order_id),
        }
    }

    fn subtotal_cents(&self) -> i32 {
        self.cart
            .iter()
            .map(|item| {
                let raw = item.unit_price_cents * item.qty;
                if item.discount_pct > 0 {
                    (raw as i64 * (100 - item.discount_pct as i64) / 100) as i32
                } else {
                    raw
                }
            })
            .sum()
    }

    fn raw_subtotal_cents(&self) -> i32 {
        self.cart.iter().map(|item| item.unit_price_cents * item.qty).sum()
    }

    fn discount_cents(&self) -> i32 {
        self.raw_subtotal_cents() - self.subtotal_cents()
    }

    fn tax_cents(&self) -> i32 {
        ((self.subtotal_cents() as i64 * 825) / 10000) as i32 // 8.25% Sales Tax
    }

    fn total_cents(&self) -> i32 {
        self.subtotal_cents() + self.tax_cents()
    }
}

fn format_cents(cents: i32) -> String {
    format!("${}.{:02}", cents / 100, (cents % 100).abs())
}

fn init_pos_database() -> Result<Connection, rusqlite::Error> {
    let conn = Connection::open("target/pos_terminal.db")?;

    // Enable high-performance WAL mode & synchronous normal
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA synchronous = NORMAL;
         PRAGMA busy_timeout = 5000;
         
         CREATE TABLE IF NOT EXISTS pos_products (
             id INTEGER PRIMARY KEY,
             sku TEXT NOT NULL UNIQUE,
             name TEXT NOT NULL,
             category TEXT NOT NULL,
             price_cents INTEGER NOT NULL,
             stock_qty INTEGER NOT NULL
         );

         CREATE TABLE IF NOT EXISTS pos_orders (
             id INTEGER PRIMARY KEY AUTOINCREMENT,
             order_no TEXT NOT NULL UNIQUE,
             created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
             subtotal_cents INTEGER NOT NULL,
             tax_cents INTEGER NOT NULL,
             discount_cents INTEGER NOT NULL,
             total_cents INTEGER NOT NULL,
             payment_method TEXT NOT NULL,
             tendered_cents INTEGER NOT NULL,
             change_cents INTEGER NOT NULL
         );

         CREATE TABLE IF NOT EXISTS pos_order_items (
             id INTEGER PRIMARY KEY AUTOINCREMENT,
             order_no TEXT NOT NULL,
             sku TEXT NOT NULL,
             name TEXT NOT NULL,
             unit_price_cents INTEGER NOT NULL,
             qty INTEGER NOT NULL,
             line_total_cents INTEGER NOT NULL
         );"
    )?;

    // Seed realistic retail products if table is empty
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM pos_products", [], |r| r.get(0))?;
    if count == 0 {
        let mut stmt = conn.prepare(
            "INSERT INTO pos_products (id, sku, name, category, price_cents, stock_qty)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)"
        )?;

        let initial_items = [
            // Beverages
            (1, "BEV-01", "Artisan Espresso (Double)", "Beverages", 350, 48),
            (2, "BEV-02", "Caramel Macchiato Latte", "Beverages", 495, 32),
            (3, "BEV-03", "Ceremonial Matcha Latte", "Beverages", 525, 24),
            (4, "BEV-04", "Nitro Cold Brew Infusion", "Beverages", 475, 40),
            (5, "BEV-05", "Sparkling Yuzu Tonic", "Beverages", 395, 18),
            // Bakery & Food
            (6, "BAK-01", "Butter Croissant (French)", "Bakery & Food", 350, 20),
            (7, "BAK-02", "Blueberry Cream Muffin", "Bakery & Food", 375, 15),
            (8, "BAK-03", "Avocado Sourdough Toast", "Bakery & Food", 850, 12),
            (9, "BAK-04", "Smoked Salmon Bagel", "Bakery & Food", 1050, 9),
            (10, "BAK-05", "Artisan Cinnamon Roll", "Bakery & Food", 425, 14),
            // Hardware & Tech
            (11, "TEC-01", "Mechanical Keycap PBT Set", "Hardware & Tech", 2800, 11),
            (12, "TEC-02", "Braided USB-C Cable (2m)", "Hardware & Tech", 1250, 35),
            (13, "TEC-03", "Ergonomic Vertical Mouse", "Hardware & Tech", 4500, 8),
            (14, "TEC-04", "Aluminum Laptop Riser", "Hardware & Tech", 3500, 6),
            (15, "TEC-05", "Extended Desk Mat (900mm)", "Hardware & Tech", 2200, 19),
            // Merchandise
            (16, "MRC-01", "Rust Direct3D Ceramic Mug", "Merchandise", 1600, 25),
            (17, "MRC-02", "Single Origin Roast (12oz)", "Merchandise", 1850, 30),
            (18, "MRC-03", "Thermal Stainless Tumbler", "Merchandise", 2400, 14),
            (19, "MRC-04", "Heavyweight Canvas Tote", "Merchandise", 1400, 22),
            (20, "MRC-05", "Enamel Hardware Logo Pin", "Merchandise", 800, 50),
        ];

        for item in initial_items {
            stmt.execute(params![item.0, item.1, item.2, item.3, item.4, item.5])?;
        }
    }

    Ok(conn)
}

fn load_products_from_db(conn: &Connection) -> Vec<DbProduct> {
    let mut stmt = conn
        .prepare("SELECT id, sku, name, category, price_cents, stock_qty FROM pos_products ORDER BY id ASC")
        .expect("Failed to prepare products query");

    let rows = stmt
        .query_map([], |row| {
            Ok(DbProduct {
                id: row.get(0)?,
                sku: row.get(1)?,
                name: row.get(2)?,
                category: row.get(3)?,
                price_cents: row.get(4)?,
                stock_qty: row.get(5)?,
            })
        })
        .expect("Query failed");

    rows.filter_map(Result::ok).collect()
}

fn main() -> Result<(), slint::PlatformError> {
    // 1. Initialize SQLite Database
    let db_conn = Rc::new(RefCell::new(
        init_pos_database().expect("Failed to initialize SQLite POS database")
    ));

    let products = load_products_from_db(&db_conn.borrow());
    let state = Rc::new(RefCell::new(PosTerminalState::new(products)));

    // 2. Initialize UI Window
    let app = PosTerminalWindow::new()?;
    app.global::<Palette>().set_dark_mode(is_system_dark_mode());

    // 3. UI Synchronization Closure
    let sync_ui = {
        let app_weak = app.as_weak();
        let state_ref = state.clone();
        move || {
            if let Some(app) = app_weak.upgrade() {
                let st = state_ref.borrow();

                // A. Filter products based on search query & active category tab
                let filtered_products: Vec<PosProduct> = st
                    .products
                    .iter()
                    .filter(|p| {
                        let cat_match = match st.current_category {
                            1 => p.category == "Beverages",
                            2 => p.category == "Bakery & Food",
                            3 => p.category == "Hardware & Tech",
                            4 => p.category == "Merchandise",
                            _ => true,
                        };
                        let q = st.search_query.trim().to_lowercase();
                        let query_match = q.is_empty()
                            || p.name.to_lowercase().contains(&q)
                            || p.sku.to_lowercase().contains(&q);
                        cat_match && query_match
                    })
                    .map(|p| PosProduct {
                        id: p.id,
                        sku: p.sku.clone().into(),
                        name: p.name.clone().into(),
                        category: p.category.clone().into(),
                        price_display: format_cents(p.price_cents).into(),
                        price_cents: p.price_cents,
                        stock_qty: p.stock_qty,
                        is_low_stock: p.stock_qty < 10,
                    })
                    .collect();

                app.set_products(Rc::new(VecModel::from(filtered_products)).into());

                // B. Map Cart items
                let cart_rows: Vec<PosCartRow> = st
                    .cart
                    .iter()
                    .map(|item| {
                        let raw = item.unit_price_cents * item.qty;
                        let line_total = if item.discount_pct > 0 {
                            (raw as i64 * (100 - item.discount_pct as i64) / 100) as i32
                        } else {
                            raw
                        };
                        PosCartRow {
                            product_id: item.product_id,
                            sku: item.sku.clone().into(),
                            name: item.name.clone().into(),
                            unit_price_display: format_cents(item.unit_price_cents).into(),
                            unit_price_cents: item.unit_price_cents,
                            qty: item.qty,
                            discount_pct: item.discount_pct,
                            line_total_display: format_cents(line_total).into(),
                            line_total_cents: line_total,
                        }
                    })
                    .collect();

                app.set_cart_rows(Rc::new(VecModel::from(cart_rows)).into());
                app.set_cart_items_count(st.cart.iter().map(|i| i.qty).sum());
                app.set_subtotal_display(format_cents(st.subtotal_cents()).into());
                app.set_tax_display(format_cents(st.tax_cents()).into());
                app.set_discount_display(format_cents(st.discount_cents()).into());
                app.set_total_display(format_cents(st.total_cents()).into());
                app.set_order_number(st.order_no.clone().into());

                // C. Tender & Change calculation
                let total = st.total_cents();
                let tendered = st.tendered_cents;
                let is_sufficient = tendered >= total && total > 0;
                let change = if is_sufficient { tendered - total } else { 0 };

                app.set_is_tender_sufficient(is_sufficient);
                app.set_change_due_display(format_cents(change).into());
                if tendered > 0 && tendered < total {
                    app.set_tender_has_error(true);
                    app.set_tender_error_text(
                        format!("Short by {}", format_cents(total - tendered)).into(),
                    );
                } else {
                    app.set_tender_has_error(false);
                    app.set_tender_error_text("".into());
                }
            }
        }
    };

    // Initial render
    sync_ui();

    // 4. Callback: Add Product to Cart
    let state_add = state.clone();
    let sync_add = sync_ui.clone();
    let app_weak_add = app.as_weak();
    app.on_add_to_cart(move |product_id| {
        let mut st = state_add.borrow_mut();
        if let Some(prod) = st.products.iter().find(|p| p.id == product_id).cloned() {
            if let Some(existing) = st.cart.iter_mut().find(|item| item.product_id == product_id) {
                existing.qty += 1;
            } else {
                st.cart.push(ActiveCartItem {
                    product_id: prod.id,
                    sku: prod.sku.clone(),
                    name: prod.name.clone(),
                    unit_price_cents: prod.price_cents,
                    qty: 1,
                    discount_pct: 0,
                });
            }
            drop(st);
            sync_add();
            if let Some(app) = app_weak_add.upgrade() {
                app.invoke_trigger_toast(
                    "success".into(),
                    "Added to Ticket".into(),
                    format!("{} added to active order.", prod.name).into(),
                );
            }
        }
    });

    // 5. Callback: Update Item Quantity (via Stepper)
    let state_qty = state.clone();
    let sync_qty = sync_ui.clone();
    app.on_update_cart_qty(move |product_id, new_qty| {
        let mut st = state_qty.borrow_mut();
        if new_qty <= 0 {
            st.cart.retain(|item| item.product_id != product_id);
        } else if let Some(item) = st.cart.iter_mut().find(|i| i.product_id == product_id) {
            item.qty = new_qty;
        }
        drop(st);
        sync_qty();
    });

    // 6. Callback: Remove Item from Cart
    let state_rm = state.clone();
    let sync_rm = sync_ui.clone();
    let app_weak_rm = app.as_weak();
    app.on_remove_from_cart(move |product_id| {
        let mut st = state_rm.borrow_mut();
        let name = st
            .cart
            .iter()
            .find(|i| i.product_id == product_id)
            .map(|i| i.name.clone())
            .unwrap_or_else(|| "Item".to_string());
        st.cart.retain(|item| item.product_id != product_id);
        drop(st);
        sync_rm();
        if let Some(app) = app_weak_rm.upgrade() {
            app.invoke_trigger_toast(
                "info".into(),
                "Removed from Ticket".into(),
                format!("{} was removed.", name).into(),
            );
        }
    });

    // 7. Callback: Apply Line Item Discount
    let state_disc = state.clone();
    let sync_disc = sync_ui.clone();
    let app_weak_disc = app.as_weak();
    app.on_apply_line_discount(move |product_id, discount_pct| {
        let mut st = state_disc.borrow_mut();
        if let Some(item) = st.cart.iter_mut().find(|i| i.product_id == product_id) {
            item.discount_pct = discount_pct;
            let name = item.name.clone();
            drop(st);
            sync_disc();
            if let Some(app) = app_weak_disc.upgrade() {
                app.invoke_trigger_toast(
                    "success".into(),
                    "Discount Applied".into(),
                    format!("{}% discount applied to {}.", discount_pct, name).into(),
                );
            }
        }
    });

    // 8. Callback: Clear Cart
    let state_clear = state.clone();
    let sync_clear = sync_ui.clone();
    let app_weak_clear = app.as_weak();
    app.on_clear_cart(move || {
        let mut st = state_clear.borrow_mut();
        st.cart.clear();
        drop(st);
        sync_clear();
        if let Some(app) = app_weak_clear.upgrade() {
            app.invoke_trigger_toast(
                "info".into(),
                "Ticket Cleared".into(),
                "All items removed from ticket.".into(),
            );
        }
    });

    // 9. Callback: Search Query Changed
    let state_search = state.clone();
    let sync_search = sync_ui.clone();
    app.on_search_changed(move |query| {
        state_search.borrow_mut().search_query = query.as_str().to_string();
        sync_search();
    });

    // 10. Callback: Category Tab Selected
    let state_cat = state.clone();
    let sync_cat = sync_ui.clone();
    let app_weak_cat = app.as_weak();
    let timer = slint::Timer::default();
    timer.start(slint::TimerMode::Repeated, std::time::Duration::from_millis(200), move || {
        if let Some(app) = app_weak_cat.upgrade() {
            let selected = app.get_selected_category_idx() as usize;
            if selected != state_cat.borrow().current_category {
                state_cat.borrow_mut().current_category = selected;
                sync_cat();
            }
        }
    });

    // 11. Callback: Quick Tender Buttons
    let state_quick = state.clone();
    let sync_quick = sync_ui.clone();
    let app_weak_quick = app.as_weak();
    app.on_set_quick_tender(move |cents| {
        let mut st = state_quick.borrow_mut();
        if cents == 0 {
            // Exact total
            st.tendered_cents = st.total_cents();
        } else {
            st.tendered_cents = cents;
        }
        let formatted = format!("{:.2}", st.tendered_cents as f64 / 100.0);
        drop(st);
        if let Some(app) = app_weak_quick.upgrade() {
            app.set_cash_tendered_input(formatted.into());
        }
        sync_quick();
    });

    // 12. Callback: Manual Tender Input Changed
    let state_tender = state.clone();
    let sync_tender = sync_ui.clone();
    app.on_tender_input_changed(move |input_val| {
        let mut st = state_tender.borrow_mut();
        let cleaned = input_val.replace('$', "").trim().to_string();
        if let Ok(dollars) = cleaned.parse::<f64>() {
            st.tendered_cents = (dollars * 100.0).round() as i32;
        } else if cleaned.is_empty() {
            st.tendered_cents = 0;
        }
        drop(st);
        sync_tender();
    });

    // 13. Callback: Finalize Checkout & Print Receipt
    let state_checkout = state.clone();
    let db_checkout = db_conn.clone();
    let app_weak_checkout = app.as_weak();
    app.on_finalize_checkout(move |payment_method| {
        let st = state_checkout.borrow();
        if st.cart.is_empty() {
            return;
        }

        let total = st.total_cents();
        let tendered = if payment_method == "Cash" {
            st.tendered_cents
        } else {
            total
        };

        if tendered < total {
            return;
        }

        let change = tendered - total;

        // Persist transaction to SQLite in-process
        let conn = db_checkout.borrow();
        let _ = conn.execute(
            "INSERT INTO pos_orders (order_no, subtotal_cents, tax_cents, discount_cents, total_cents, payment_method, tendered_cents, change_cents)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                st.order_no,
                st.subtotal_cents(),
                st.tax_cents(),
                st.discount_cents(),
                total,
                payment_method.as_str(),
                tendered,
                change
            ],
        );

        // Record line items and decrement stock
        for item in &st.cart {
            let line_total = item.unit_price_cents * item.qty * (100 - item.discount_pct) / 100;
            let _ = conn.execute(
                "INSERT INTO pos_order_items (order_no, sku, name, unit_price_cents, qty, line_total_cents)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![st.order_no, item.sku, item.name, item.unit_price_cents, item.qty, line_total],
            );
            let _ = conn.execute(
                "UPDATE pos_products SET stock_qty = MAX(0, stock_qty - ?1) WHERE id = ?2",
                params![item.qty, item.product_id],
            );
        }

        // Map receipt items
        let receipt_items: Vec<ReceiptItem> = st
            .cart
            .iter()
            .map(|item| {
                let line_total = item.unit_price_cents * item.qty * (100 - item.discount_pct) / 100;
                ReceiptItem {
                    name: item.name.clone().into(),
                    sku: item.sku.clone().into(),
                    qty: item.qty,
                    unit_price: format_cents(item.unit_price_cents).into(),
                    line_total: format_cents(line_total).into(),
                }
            })
            .collect();

        let order_saved = st.order_no.clone();
        drop(st);

        if let Some(app) = app_weak_checkout.upgrade() {
            app.set_receipt_items(Rc::new(VecModel::from(receipt_items)).into());
            app.set_receipt_payment_method(payment_method.clone());
            app.set_receipt_tendered(format_cents(tendered).into());
            app.set_receipt_change(format_cents(change).into());
            app.set_show_checkout_modal(false);
            app.set_show_receipt_modal(true);
            app.invoke_trigger_toast(
                "success".into(),
                "Transaction Finalized".into(),
                format!("Order {} settled and committed to SQLite.", order_saved).into(),
            );
        }
    });

    // 14. Callback: Start New Order
    let state_new = state.clone();
    let db_new = db_conn.clone();
    let sync_new = sync_ui.clone();
    let app_weak_new = app.as_weak();
    app.on_start_new_order(move || {
        let mut st = state_new.borrow_mut();
        st.cart.clear();
        st.tendered_cents = 0;
        let new_id = ORDER_COUNTER.fetch_add(1, Ordering::SeqCst);
        st.order_no = format!("ORD-{}", new_id);

        // Reload fresh stock levels from SQLite
        st.products = load_products_from_db(&db_new.borrow());
        drop(st);

        if let Some(app) = app_weak_new.upgrade() {
            app.set_show_receipt_modal(false);
            app.set_show_checkout_modal(false);
            app.set_cash_tendered_input("".into());
        }
        sync_new();
    });

    app.run()
}

