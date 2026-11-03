let currentCollection = "users";
    let currentWorkspaceSubView = "records"; // 'records' | 'settings'
    let collections = [];
    let currentRecords = [];
    let selectedRecordIds = new Set();
    let currentPage = 1;
    let currentSort = "";
    let currentFilter = "";
    let sseSource = null;
    let activeDbDriver = "sqlite";
    let currentSchemaDbEngine = "sqlite";
    let inlineSchemaFields = [];

    const DB_ENGINE_TYPES = {
      sqlite: {
        name: "SQLite",
        types: [
          { type: 'text', sqlType: 'TEXT', label: 'Text', sub: 'TEXT', icon: 'fa-font' },
          { type: 'number', sqlType: 'INTEGER', label: 'Integer', sub: 'INTEGER', icon: 'fa-hashtag' },
          { type: 'number', sqlType: 'REAL', label: 'Real / Float', sub: 'REAL', icon: 'fa-calculator' },
          { type: 'bool', sqlType: 'INTEGER', label: 'Bool', sub: 'BOOLEAN', icon: 'fa-toggle-on' },
          { type: 'date', sqlType: 'TEXT', label: 'Datetime', sub: 'DATETIME', icon: 'fa-calendar-days' },
          { type: 'json', sqlType: 'TEXT', label: 'JSON', sub: 'JSON', icon: 'fa-code' },
          { type: 'file', sqlType: 'TEXT', label: 'File / Blob', sub: 'BLOB', icon: 'fa-paperclip' },
          { type: 'relation', sqlType: 'INTEGER', label: 'Relation (FK)', sub: 'FOREIGN KEY', icon: 'fa-diagram-project' },
          { type: 'email', sqlType: 'TEXT', label: 'Email', sub: 'TEXT', icon: 'fa-envelope' },
          { type: 'select', sqlType: 'TEXT', label: 'Select', sub: 'TEXT', icon: 'fa-list-check' }
        ]
      },
      postgres: {
        name: "PostgreSQL",
        types: [
          { type: 'text', sqlType: 'TEXT', label: 'Text', sub: 'TEXT', icon: 'fa-font' },
          { type: 'text', sqlType: 'VARCHAR(255)', label: 'Varchar', sub: 'VARCHAR', icon: 'fa-font' },
          { type: 'number', sqlType: 'INTEGER', label: 'Integer', sub: 'INT4', icon: 'fa-hashtag' },
          { type: 'number', sqlType: 'BIGINT', label: 'BigInt', sub: 'INT8', icon: 'fa-hashtag' },
          { type: 'number', sqlType: 'DOUBLE PRECISION', label: 'Numeric / Float', sub: 'NUMERIC', icon: 'fa-calculator' },
          { type: 'bool', sqlType: 'BOOLEAN', label: 'Bool', sub: 'BOOLEAN', icon: 'fa-toggle-on' },
          { type: 'date', sqlType: 'TIMESTAMPTZ', label: 'Timestamp (TZ)', sub: 'TIMESTAMPTZ', icon: 'fa-calendar-days' },
          { type: 'json', sqlType: 'JSONB', label: 'JSONB', sub: 'BINARY JSON', icon: 'fa-code' },
          { type: 'text', sqlType: 'UUID', label: 'UUID', sub: 'UUID', icon: 'fa-fingerprint' },
          { type: 'file', sqlType: 'BYTEA', label: 'File / Bytea', sub: 'BYTEA', icon: 'fa-paperclip' },
          { type: 'relation', sqlType: 'BIGINT', label: 'Relation (FK)', sub: 'FOREIGN KEY', icon: 'fa-diagram-project' },
          { type: 'email', sqlType: 'VARCHAR(255)', label: 'Email', sub: 'VARCHAR', icon: 'fa-envelope' },
          { type: 'select', sqlType: 'VARCHAR(100)', label: 'Select (Enum)', sub: 'VARCHAR', icon: 'fa-list-check' }
        ]
      },
      mysql: {
        name: "MySQL",
        types: [
          { type: 'text', sqlType: 'VARCHAR(255)', label: 'Varchar', sub: 'VARCHAR', icon: 'fa-font' },
          { type: 'text', sqlType: 'TEXT', label: 'Text', sub: 'TEXT', icon: 'fa-font' },
          { type: 'number', sqlType: 'INT', label: 'Integer', sub: 'INT', icon: 'fa-hashtag' },
          { type: 'number', sqlType: 'BIGINT', label: 'BigInt', sub: 'BIGINT', icon: 'fa-hashtag' },
          { type: 'number', sqlType: 'DECIMAL(12,2)', label: 'Decimal', sub: 'DECIMAL', icon: 'fa-calculator' },
          { type: 'bool', sqlType: 'TINYINT(1)', label: 'Bool', sub: 'TINYINT(1)', icon: 'fa-toggle-on' },
          { type: 'date', sqlType: 'DATETIME', label: 'DateTime', sub: 'DATETIME', icon: 'fa-calendar-days' },
          { type: 'json', sqlType: 'JSON', label: 'JSON', sub: 'JSON', icon: 'fa-code' },
          { type: 'file', sqlType: 'LONGBLOB', label: 'File / Blob', sub: 'BLOB', icon: 'fa-paperclip' },
          { type: 'relation', sqlType: 'BIGINT', label: 'Relation (FK)', sub: 'FOREIGN KEY', icon: 'fa-diagram-project' },
          { type: 'email', sqlType: 'VARCHAR(255)', label: 'Email', sub: 'VARCHAR', icon: 'fa-envelope' },
          { type: 'select', sqlType: 'VARCHAR(100)', label: 'Select (Enum)', sub: 'VARCHAR', icon: 'fa-list-check' }
        ]
      },
      mariadb: {
        name: "MariaDB",
        types: [
          { type: 'text', sqlType: 'VARCHAR(255)', label: 'Varchar', sub: 'VARCHAR', icon: 'fa-font' },
          { type: 'text', sqlType: 'TEXT', label: 'Text', sub: 'TEXT', icon: 'fa-font' },
          { type: 'number', sqlType: 'INT', label: 'Integer', sub: 'INT', icon: 'fa-hashtag' },
          { type: 'number', sqlType: 'BIGINT', label: 'BigInt', sub: 'BIGINT', icon: 'fa-hashtag' },
          { type: 'number', sqlType: 'DECIMAL(12,2)', label: 'Decimal', sub: 'DECIMAL', icon: 'fa-calculator' },
          { type: 'bool', sqlType: 'TINYINT(1)', label: 'Bool', sub: 'TINYINT(1)', icon: 'fa-toggle-on' },
          { type: 'date', sqlType: 'DATETIME', label: 'DateTime', sub: 'DATETIME', icon: 'fa-calendar-days' },
          { type: 'json', sqlType: 'JSON', label: 'JSON', sub: 'JSON', icon: 'fa-code' },
          { type: 'file', sqlType: 'LONGBLOB', label: 'File / Blob', sub: 'BLOB', icon: 'fa-paperclip' },
          { type: 'relation', sqlType: 'BIGINT', label: 'Relation (FK)', sub: 'FOREIGN KEY', icon: 'fa-diagram-project' },
          { type: 'email', sqlType: 'VARCHAR(255)', label: 'Email', sub: 'VARCHAR', icon: 'fa-envelope' },
          { type: 'select', sqlType: 'VARCHAR(100)', label: 'Select (Enum)', sub: 'VARCHAR', icon: 'fa-list-check' }
        ]
      }
    };

    function getSqlTypeForField(f, engine) {
      if (f.options && (f.options.sqlType || f.options.nativeType)) {
        return f.options.sqlType || f.options.nativeType;
      }
      const eng = engine || currentSchemaDbEngine || activeDbDriver || 'sqlite';
      if (eng === 'sqlite') {
        if (f.type === 'number') return 'REAL';
        if (f.type === 'bool') return 'INTEGER';
        if (f.type === 'relation') return 'INTEGER';
        return 'TEXT';
      } else if (eng === 'postgres') {
        if (f.type === 'number') return 'DOUBLE PRECISION';
        if (f.type === 'bool') return 'BOOLEAN';
        if (f.type === 'json') return 'JSONB';
        if (f.type === 'date') return 'TIMESTAMPTZ';
        if (f.type === 'relation') return 'BIGINT';
        if (f.type === 'file') return 'BYTEA';
        return f.unique ? 'VARCHAR(255)' : 'TEXT';
      } else {
        if (f.type === 'number') return 'DOUBLE';
        if (f.type === 'bool') return 'TINYINT(1)';
        if (f.type === 'json') return 'JSON';
        if (f.type === 'date') return 'DATETIME';
        if (f.type === 'relation') return 'BIGINT';
        return f.unique ? 'VARCHAR(255)' : 'TEXT';
      }
    }

    function formatRelationFieldName(targetTable, existingFields = [], currentIdx = -1) {
      if (!targetTable) return 'relation_id';
      let baseName = targetTable.toLowerCase().trim();
      if (baseName.endsWith('ies')) {
        baseName = baseName.slice(0, -3) + 'y';
      } else if (baseName.endsWith('sses')) {
        baseName = baseName.slice(0, -2);
      } else if (baseName.endsWith('xes') || baseName.endsWith('shes') || baseName.endsWith('ches')) {
        baseName = baseName.slice(0, -2);
      } else if (baseName.endsWith('s') && !baseName.endsWith('ss') && !baseName.endsWith('us') && !baseName.endsWith('is')) {
        baseName = baseName.slice(0, -1);
      }
      const candidate = `${baseName}_id`;
      if (!Array.isArray(existingFields) || existingFields.length === 0) return candidate;

      const isTaken = existingFields.some((f, i) => i !== currentIdx && f && f.name === candidate);
      if (!isTaken) return candidate;

      let counter = 2;
      while (existingFields.some((f, i) => i !== currentIdx && f && f.name === `${baseName}_id_${counter}`)) {
        counter++;
      }
      return `${baseName}_id_${counter}`;
    }

    function getVarcharLength(f) {
      if (f.options?.length) return parseInt(f.options.length, 10) || 255;
      if (f.options?.maxLength) return parseInt(f.options.maxLength, 10) || 255;
      const match = (f.options?.sqlType || '').match(/VARCHAR\((\d+)\)/i);
      if (match) return parseInt(match[1], 10);
      return 255;
    }

    function isVarcharType(f, engine) {
      if (f.options?.sqlType && f.options.sqlType.toUpperCase().startsWith('VARCHAR')) return true;
      const resolved = getSqlTypeForField(f, engine);
      return resolved.toUpperCase().startsWith('VARCHAR');
    }

    function updateInlineVarcharLength(idx, val) {
      if (!inlineSchemaFields[idx]) return;
      const num = parseInt(val, 10);
      const len = (!isNaN(num) && num > 0) ? num : 255;
      inlineSchemaFields[idx].options = inlineSchemaFields[idx].options || {};
      inlineSchemaFields[idx].options.length = len;
      inlineSchemaFields[idx].options.sqlType = `VARCHAR(${len})`;
      const badge = document.getElementById(`inlineSqlTypeBadge_${idx}`);
      if (badge) badge.textContent = `SQL: VARCHAR(${len})`;
    }

    function updateNewTableVarcharLength(idx, val) {
      if (!newTableFields[idx]) return;
      const num = parseInt(val, 10);
      const len = (!isNaN(num) && num > 0) ? num : 255;
      newTableFields[idx].options = newTableFields[idx].options || {};
      newTableFields[idx].options.length = len;
      newTableFields[idx].options.sqlType = `VARCHAR(${len})`;
      const badge = document.getElementById(`newTableSqlTypeBadge_${idx}`);
      if (badge) badge.textContent = `SQL: VARCHAR(${len})`;
    }

    function setSchemaDbEngine(driver) {
      // Locked to detected database engine - manual switching disabled
      currentSchemaDbEngine = activeDbDriver || 'sqlite';
      renderSchemaDataTypeButtons();
      renderInlineSchemaFields();
      renderNewTableFields();
    }

    function renderSchemaDataTypeButtons() {
      const engine = activeDbDriver || currentSchemaDbEngine || 'sqlite';
      currentSchemaDbEngine = engine;
      const cfg = DB_ENGINE_TYPES[engine] || DB_ENGINE_TYPES['sqlite'];

      const inlineActiveLabel = document.getElementById('inlineActiveDbLabel');
      if (inlineActiveLabel) inlineActiveLabel.textContent = cfg.name.toUpperCase();
      const newColActiveLabel = document.getElementById('newColActiveDbLabel');
      if (newColActiveLabel) newColActiveLabel.textContent = cfg.name.toUpperCase();

      const inlineContainer = document.getElementById('inlineDataTypeButtons');
      if (inlineContainer) {
        inlineContainer.innerHTML = cfg.types.map(t => {
          const isRel = t.type === 'relation';
          return `
            <button type="button" onclick="addInlineSchemaField('${t.type}', '${t.sqlType}')" class="px-2.5 py-1.5 rounded-lg ${isRel ? 'bg-teal-50 border-teal-300 text-teal-800 hover:bg-teal-100' : 'bg-white border-slate-200 text-slate-700 hover:border-emerald-400 hover:text-emerald-700'} border text-xs font-mono font-semibold transition flex items-center space-x-1.5 shadow-2xs" title="${t.sub} in ${cfg.name}">
              <i class="${FIELD_ICONS[t.type] || 'fa-solid fa-font'} text-[10px] ${isRel ? 'text-teal-600' : 'text-emerald-600'}"></i>
              <span>${t.label}</span>
              <span class="text-[9px] px-1 py-0.2 rounded bg-slate-100 text-slate-500 font-bold uppercase">${t.sub}</span>
            </button>
          `;
        }).join('');
      }

      const newColContainer = document.getElementById('newColDataTypeButtons');
      if (newColContainer) {
        newColContainer.innerHTML = cfg.types.map(t => {
          const isRel = t.type === 'relation';
          return `
            <button type="button" onclick="addNewColField('${t.type}', '${t.sqlType}')" class="px-2.5 py-1.5 rounded-lg ${isRel ? 'bg-teal-50 border-teal-300 text-teal-800 hover:bg-teal-100' : 'bg-white border-slate-200 text-slate-700 hover:border-emerald-400 hover:text-emerald-700'} border text-xs font-mono font-semibold transition flex items-center space-x-1.5 shadow-2xs" title="${t.sub} in ${cfg.name}">
              <i class="${FIELD_ICONS[t.type] || 'fa-solid fa-font'} text-[10px] ${isRel ? 'text-teal-600' : 'text-emerald-600'}"></i>
              <span>${t.label}</span>
              <span class="text-[9px] px-1 py-0.2 rounded bg-slate-100 text-slate-500 font-bold uppercase">${t.sub}</span>
            </button>
          `;
        }).join('');
      }
    }

    // Modal states
    let editingRecordId = null;
    let isRecordJsonMode = false;
    let isEditingCollection = false;
    let tempSchemaFields = [];

    const FIELD_ICONS = {
      text: 'fa-solid fa-font',
      number: 'fa-solid fa-hashtag',
      bool: 'fa-solid fa-toggle-on',
      email: 'fa-regular fa-envelope',
      url: 'fa-solid fa-link',
      date: 'fa-regular fa-calendar',
      select: 'fa-solid fa-list-check',
      json: 'fa-solid fa-code',
      file: 'fa-regular fa-file',
      relation: 'fa-solid fa-diagram-project'
    };

    function showToast(msg, type = 'success') {
      const container = document.getElementById('toastContainer');
      const el = document.createElement('div');
      const bg = type === 'error'
        ? 'bg-rose-600 text-white'
        : 'bg-emerald-950 text-white border border-emerald-500/30';
      el.className = `${bg} px-4 py-3 rounded-2xl shadow-xl text-xs font-semibold flex items-center space-x-2.5 pointer-events-auto transition duration-200`;
      el.innerHTML = `<i class="fa-solid ${type === 'error' ? 'fa-circle-exclamation' : 'fa-circle-check text-emerald-400'}"></i> <span>${msg}</span>`;
      container.appendChild(el);
      setTimeout(() => el.remove(), 3200);
    }

    function openQuickGuideModal() {
      document.getElementById('quickGuideModal').classList.remove('hidden');
    }

    // Automatically attach admin superuser JWT token to all API requests made within the Dashboard
    const _originalFetch = window.fetch;
    window.fetch = async function (url, options = {}) {
      const opts = Object.assign({}, options);
      const token = localStorage.getItem('padi_admin_token');
      if (token && typeof url === 'string' && url.startsWith('/api/')) {
        const headers = new Headers(opts.headers || {});
        if (!headers.has('Authorization')) {
          headers.set('Authorization', `Bearer ${token}`);
        }
        opts.headers = headers;
      }
      return _originalFetch(url, opts);
    };

    async function init() {
      const token = localStorage.getItem('padi_admin_token');
      const email = localStorage.getItem('padi_admin_email');

      if (token && email) {
        showDashboard(email);
      } else {
        await checkHasAdmins();
      }
    }

    async function loadHealthInfo() {
      try {
        const res = await fetch('/api/health');
        const data = await res.json();
        const db = data.data?.database || data.debug?.database || data.item?.database || 'sqlite';
        activeDbDriver = db;
        currentSchemaDbEngine = db;
        const headerBadge = document.getElementById('headerDbBadge');
        if (headerBadge) headerBadge.textContent = String(db).toUpperCase();
        const settingsBadge = document.getElementById('settingsDbBadge');
        if (settingsBadge) settingsBadge.textContent = String(db).toUpperCase() + ' Engine';
        renderSchemaDataTypeButtons();
      } catch (_) {}
    }

    function switchToLoginForm() {
      document.getElementById('setupAdminForm').classList.add('hidden');
      document.getElementById('loginAdminForm').classList.remove('hidden');
      document.getElementById('authAlert').classList.add('hidden');
    }

    function switchToSetupForm() {
      document.getElementById('loginAdminForm').classList.add('hidden');
      document.getElementById('setupAdminForm').classList.remove('hidden');
      document.getElementById('authAlert').classList.add('hidden');
    }

    async function checkHasAdmins() {
      try {
        const res = await fetch('/api/admins/has-admins');
        let hasAdmins = false;
        if (res.ok) {
          const data = await res.json();
          hasAdmins = Boolean(data.item?.hasAdmins ?? data.hasAdmins);
        } else {
          hasAdmins = true;
        }
        document.getElementById('authScreen').classList.remove('hidden');
        document.getElementById('dashboardApp').classList.add('hidden');

        if (hasAdmins) {
          document.getElementById('loginAdminForm').classList.remove('hidden');
          document.getElementById('setupAdminForm').classList.add('hidden');
        } else {
          document.getElementById('setupAdminForm').classList.remove('hidden');
          document.getElementById('loginAdminForm').classList.add('hidden');
        }
      } catch (err) {
        document.getElementById('authScreen').classList.remove('hidden');
        document.getElementById('dashboardApp').classList.add('hidden');
        document.getElementById('loginAdminForm').classList.remove('hidden');
        document.getElementById('setupAdminForm').classList.add('hidden');
        showAuthAlert('Unable to connect to PadiBase server: ' + err.message);
      }
    }

    function showAuthAlert(msg) {
      const el = document.getElementById('authAlert');
      el.textContent = msg;
      el.classList.remove('hidden');
    }

    async function handleFirstAdminCreate() {
      const email = document.getElementById('setupEmail').value.trim();
      const password = document.getElementById('setupPassword').value;
      const confirm = document.getElementById('setupPasswordConfirm').value;

      if (!email || !password) return showAuthAlert('Email and password are required');
      if (password.length < 8) return showAuthAlert('Password must be at least 8 characters');
      if (password !== confirm) return showAuthAlert('Passwords do not match');

      try {
        const res = await fetch('/api/admins', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ email, password })
        });
        const data = await res.json();
        if (res.ok) {
          await doLogin(email, password);
        } else {
          showAuthAlert(data.message || 'Failed to create superuser account');
        }
      } catch (err) {
        showAuthAlert(err.message);
      }
    }

    async function handleAdminLogin() {
      const email = document.getElementById('loginEmail').value.trim();
      const password = document.getElementById('loginPassword').value;
      if (!email || !password) return showAuthAlert('Email and password are required');
      await doLogin(email, password);
    }

    async function doLogin(identity, password) {
      try {
        const res = await fetch('/api/admins/auth-with-password', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ identity, password })
        });
        const data = await res.json();
        const authData = data.item || data;
        if (res.ok && authData.token) {
          localStorage.setItem('padi_admin_token', authData.token);
          localStorage.setItem('padi_admin_email', authData.admin.email);
          showDashboard(authData.admin.email);
        } else {
          showAuthAlert(data.message || 'Invalid email or password');
        }
      } catch (err) {
        showAuthAlert(err.message);
      }
    }

    function showDashboard(email) {
      document.getElementById('authScreen').classList.add('hidden');
      document.getElementById('dashboardApp').classList.remove('hidden');
      document.getElementById('userEmail').textContent = email;
      document.getElementById('adminAvatarBtn').textContent = email.slice(0, 2).toUpperCase();

      loadHealthInfo();
      loadCollections();
      loadAppSettings();
      initSSE();
    }

    function handleLogout() {
      localStorage.removeItem('padi_admin_token');
      localStorage.removeItem('padi_admin_email');
      if (sseSource) sseSource.close();
      checkHasAdmins();
    }

    // ================= NAVIGATION & VIEWS =================
    function toggleMobileSidebar(force) {
      const sidebar = document.getElementById('schemaSidebar');
      const backdrop = document.getElementById('sidebarBackdrop');
      if (!sidebar || !backdrop) return;
      const isCurrentlyOpen = !sidebar.classList.contains('-translate-x-full');
      const shouldOpen = force !== undefined ? force : !isCurrentlyOpen;
      if (shouldOpen) {
        sidebar.classList.remove('-translate-x-full');
        sidebar.classList.add('translate-x-0');
        backdrop.classList.remove('hidden');
      } else {
        sidebar.classList.add('-translate-x-full');
        sidebar.classList.remove('translate-x-0');
        backdrop.classList.add('hidden');
      }
    }

    function switchMainView(view) {
      if (view !== 'collections' && window.innerWidth < 1024) {
        toggleMobileSidebar(false);
      }
      const views = ['collections', 'endpoints', 'logs', 'settings'];
      views.forEach(v => {
        const el = document.getElementById(`${v}View`);
        if (el) el.classList.toggle('hidden', v !== view);
        const btn = document.getElementById(`rail${v.charAt(0).toUpperCase() + v.slice(1)}Btn`);
        if (btn) {
          if (v === view) {
            btn.className = "w-full flex items-center space-x-3 px-3 py-2.5 rounded-xl text-xs font-bold transition bg-emerald-600 text-white shadow-sm relative";
          } else {
            btn.className = "w-full flex items-center space-x-3 px-3 py-2.5 rounded-xl text-xs font-semibold text-slate-400 hover:text-white hover:bg-slate-900 transition relative";
          }
        }
      });

      if (view === 'endpoints') {
        updateCustomEpUrlPreview();
        loadCustomEndpoints();
        populateQbTables();
      } else if (view === 'settings') {
        renderQuickMatrixTable();
        loadAppSettings();
        loadBackups();
        loadAdminAccounts();
        document.getElementById('exportCollectionsPre').textContent = JSON.stringify(collections, null, 2);
      }
    }

    // ================= DYNAMIC CUSTOM SQL ENDPOINTS BUILDER =================
    let customEndpoints = [];
    let currentCustomEpId = null;

    function isOverrideEndpointPath(rawPath) {
      if (!rawPath) return false;
      const clean = rawPath.trim().replace(/^\/+/, '').toLowerCase();

      // Core built-in system routes
      if (clean === 'api/health' || clean === 'api/tables' || clean === 'api/collections' || clean === 'api/realtime') {
        return true;
      }

      // Built-in table default routes (e.g. api/tables/banks/records or api/collections/banks/records)
      for (const c of (collections || [])) {
        const colName = (c.name || '').toLowerCase();
        if (!colName) continue;

        const prefixTables = `api/tables/${colName}/`;
        const prefixCollections = `api/collections/${colName}/`;
        if (clean.startsWith(prefixTables) || clean.startsWith(prefixCollections)) {
          const action = clean.startsWith(prefixTables) 
            ? clean.substring(prefixTables.length) 
            : clean.substring(prefixCollections.length);
          if (
            action === 'records' ||
            action === 'records/all' ||
            action.startsWith('records/') ||
            action === 'delete-all' ||
            action === 'truncate' ||
            action === 'import' ||
            action === 'export' ||
            action === 'auth-with-password' ||
            action === 'login' ||
            action === 'register' ||
            action === 'forgot-password' ||
            action === 'request-password-reset' ||
            action === 'reset-password' ||
            action === 'confirm-password-reset' ||
            action === 'change-password' ||
            action === 'auth-refresh' ||
            action === 'me' ||
            action === 'request-verification' ||
            action === 'confirm-verification'
          ) {
            return true;
          }
        }

        // Legacy / framework full dump route: api/{collection}/all
        if (clean === `api/${colName}/all`) {
          return true;
        }
      }

      return false;
    }

    function onCustomEpPrefixChange(val) {
      const customInput = document.getElementById('epCustomPrefixInput');
      if (customInput) {
        if (val === 'custom' || val === '__custom__') {
          customInput.classList.remove('hidden');
          customInput.focus();
        } else {
          customInput.classList.add('hidden');
        }
      }
      updateCustomEpUrlPreview();
    }

    function getSelectedPrefix() {
      const sel = document.getElementById('epPrefixSelect')?.value;
      if (sel === 'custom' || sel === '__custom__') {
        let p = (document.getElementById('epCustomPrefixInput')?.value || '').trim();
        if (!p) return '';
        if (!p.endsWith('/')) p = p + '/';
        return p;
      }
      return sel !== undefined && sel !== null ? sel : '';
    }

    function getCombinedEndpointPath() {
      let rawSlug = (document.getElementById('epPathInput')?.value || '').trim();
      if (!rawSlug) return '';
      rawSlug = rawSlug.replace(/^\/+/, '');

      const prefix = getSelectedPrefix(); // e.g. "", "v1/", "custom/", "collections/", "/", or custom

      // If user selected root "/" (outside /api)
      if (prefix === '/') {
        return rawSlug;
      }

      // If custom prefix outside /api (starts with '/')
      if (prefix.startsWith('/') && !prefix.startsWith('/api')) {
        return `${prefix.replace(/^\/+/, '')}${rawSlug}`;
      }

      // If user typed full path starting with "api/"
      if (rawSlug.startsWith('api/')) {
        return rawSlug;
      }

      // Default BaaS API routes: always prepend "api/"
      const subPrefix = prefix.replace(/^\/?api\/?/, '').replace(/^\/+/, '');
      const cleanSub = subPrefix ? (subPrefix.endsWith('/') ? subPrefix : subPrefix + '/') : '';

      // Don't duplicate if user already typed the sub-prefix (e.g. typed "collections/banks" with prefix "collections/")
      if (cleanSub && rawSlug.startsWith(cleanSub)) {
        return `api/${rawSlug}`;
      }

      return `api/${cleanSub}${rawSlug}`;
    }

    function resolveEndpointFullPath(rawPath) {
      const clean = (rawPath || '').trim().replace(/^\/+/, '');
      if (!clean) return '/api/endpoint';
      return `/${clean}`;
    }

    function updateCustomEpUrlPreview() {
      const combined = getCombinedEndpointPath();
      const prefix = getSelectedPrefix();

      let fullPath;
      if (combined) {
        fullPath = `/${combined}`;
      } else {
        // Placeholder when path input is empty
        if (prefix === '/') {
          fullPath = '/endpoint';
        } else if (prefix) {
          fullPath = `/api/${prefix.replace(/^\/?api\/?/, '')}endpoint`.replace(/\/+/g, '/');
        } else {
          fullPath = '/api/endpoint';
        }
      }

      const isOverride = isOverrideEndpointPath(combined || fullPath);
      const el = document.getElementById('epLiveUrlPreview');
      if (el) {
        el.textContent = `${window.location.origin}${fullPath}`;
      }
      const overrideBadge = document.getElementById('epOverrideModeBadge');
      if (overrideBadge) {
        overrideBadge.classList.toggle('hidden', !isOverride);
      }
    }

    function applyOverrideEndpointPreset(preset) {
      if (!preset) return;
      const col = currentCollection || 'users';
      const nameEl = document.getElementById('epNameInput');
      const methodEl = document.getElementById('epMethodInput');
      const prefixEl = document.getElementById('epPrefixSelect');
      const customPrefixEl = document.getElementById('epCustomPrefixInput');
      const pathEl = document.getElementById('epPathInput');
      const sqlEl = document.getElementById('epSqlInput');
      const txEl = document.getElementById('epTransactionInput');
      const paramsEl = document.getElementById('epTestParamsInput');

      if (prefixEl) prefixEl.value = '/';
      if (customPrefixEl) customPrefixEl.classList.add('hidden');

      if (preset === 'list_records') {
        nameEl.value = `Override List ${col}`;
        methodEl.value = 'GET';
        pathEl.value = `api/collections/${col}/records`;
        txEl.checked = false;
        sqlEl.value = `SELECT * FROM \`${col}\` ORDER BY \`id\` DESC LIMIT 50;`;
        if (paramsEl) paramsEl.value = '{}';
      } else if (preset === 'all_records') {
        nameEl.value = `Override All ${col}`;
        methodEl.value = 'GET';
        pathEl.value = `api/${col}/all`;
        txEl.checked = false;
        sqlEl.value = `SELECT * FROM \`${col}\` ORDER BY \`id\` DESC;`;
        if (paramsEl) paramsEl.value = '{}';
      } else if (preset === 'get_record') {
        nameEl.value = `Override Detail ${col}`;
        methodEl.value = 'GET';
        pathEl.value = `api/collections/${col}/records/{id}`;
        txEl.checked = false;
        sqlEl.value = `SELECT * FROM \`${col}\` WHERE \`id\` = :id LIMIT 1;`;
        if (paramsEl) paramsEl.value = '{\n  "id": "1"\n}';
      } else if (preset === 'create_record') {
        nameEl.value = `Override Create ${col}`;
        methodEl.value = 'POST';
        pathEl.value = `api/collections/${col}/records`;
        txEl.checked = true;
        sqlEl.value = `SELECT * FROM \`${col}\` ORDER BY \`id\` DESC LIMIT 1;`;
      } else if (preset === 'update_record') {
        nameEl.value = `Override Update ${col}`;
        methodEl.value = 'PATCH';
        pathEl.value = `api/collections/${col}/records/{id}`;
        txEl.checked = true;
        sqlEl.value = `UPDATE \`${col}\` SET \`updated\` = CURRENT_TIMESTAMP WHERE \`id\` = :id;\nSELECT * FROM \`${col}\` WHERE \`id\` = :id LIMIT 1;`;
      } else if (preset === 'delete_record') {
        nameEl.value = `Override Delete ${col}`;
        methodEl.value = 'DELETE';
        pathEl.value = `api/collections/${col}/records/{id}`;
        txEl.checked = true;
        sqlEl.value = `DELETE FROM \`${col}\` WHERE \`id\` = :id;`;
      }

      document.getElementById('epOverridePresetSelect').value = '';
      updateCustomEpUrlPreview();
    }

    async function loadCustomEndpoints(selectIdOrPath) {
      try {
        const res = await fetch('/api/endpoints');
        const data = await res.json();
        customEndpoints = data.item || data.items || (Array.isArray(data) ? data : []);
        const badge = document.getElementById('customEpCountBadge');
        if (badge) badge.textContent = customEndpoints.length;
        renderCustomEndpointsList();

        if (selectIdOrPath) {
          const found = customEndpoints.find(e => e.id === selectIdOrPath || e.path === selectIdOrPath);
          if (found) {
            selectCustomEndpoint(found.id);
            return;
          }
        }
        if (!currentCustomEpId && customEndpoints.length > 0) {
          selectCustomEndpoint(customEndpoints[0].id);
        } else if (customEndpoints.length === 0) {
          startNewCustomEndpoint();
        }
      } catch (e) {
        console.error('Failed to load custom endpoints:', e);
      }
    }

    function renderCustomEndpointsList() {
      const container = document.getElementById('customEndpointsList');
      if (!container) return;
      const kw = (document.getElementById('customEpSearchInput')?.value || '').toLowerCase();
      const filtered = customEndpoints.filter(e =>
        (e.name || '').toLowerCase().includes(kw) || (e.path || '').toLowerCase().includes(kw)
      );

      if (filtered.length === 0) {
        container.innerHTML = `<div class="p-4 text-center text-xs text-slate-400">No custom endpoints yet. Click <strong>+ New API</strong> to create one.</div>`;
        return;
      }

      const methodColors = {
        GET: 'bg-emerald-100 text-emerald-800',
        POST: 'bg-sky-100 text-sky-800',
        PUT: 'bg-amber-100 text-amber-800',
        PATCH: 'bg-amber-100 text-amber-800',
        DELETE: 'bg-rose-100 text-rose-800',
        ANY: 'bg-purple-100 text-purple-800'
      };

      container.innerHTML = filtered.map(ep => {
        const isActive = ep.id === currentCustomEpId;
        const mColor = methodColors[ep.method] || 'bg-slate-100 text-slate-700';
        const fullDisplayPath = resolveEndpointFullPath(ep.path);
        const isOverride = isOverrideEndpointPath(ep.path);
        return `
          <button type="button" onclick="selectCustomEndpoint('${ep.id}')" class="w-full text-left p-2.5 rounded-xl border transition flex items-center justify-between ${isActive ? 'bg-emerald-600 text-white border-emerald-600 shadow-sm font-bold' : 'bg-white hover:bg-emerald-50/70 text-slate-700 border-slate-100 font-semibold'}">
            <div class="min-w-0 pr-2">
              <div class="text-xs truncate flex items-center gap-1.5">
                <span class="truncate">${ep.name || ep.path}</span>
                ${isOverride ? `<span class="px-1.5 py-0.2 rounded text-[8px] font-extrabold ${isActive ? 'bg-amber-300 text-amber-950' : 'bg-amber-100 text-amber-800'}">OVERRIDE</span>` : ''}
              </div>
              <div class="text-[10px] font-mono truncate ${isActive ? 'text-emerald-100' : 'text-slate-400'}">${fullDisplayPath}</div>
            </div>
            <span class="px-2 py-0.5 rounded-md text-[9px] font-mono font-extrabold shrink-0 ${isActive ? 'bg-white/20 text-white' : mColor}">${ep.method || 'GET'}</span>
          </button>
        `;
      }).join('');
    }

    function startNewCustomEndpoint() {
      currentCustomEpId = null;
      renderCustomEndpointsList();
      const alertEl = document.getElementById('customEpAlert');
      if (alertEl) alertEl.classList.add('hidden');

      document.getElementById('customEpFormTitle').textContent = 'Create Custom SQL Endpoint';
      document.getElementById('saveCustomEpBtnText').textContent = 'Create Endpoint';
      document.getElementById('deleteCustomEpBtn').classList.add('hidden');

      document.getElementById('epNameInput').value = '';
      document.getElementById('epMethodInput').value = 'GET';
      const prefixEl = document.getElementById('epPrefixSelect');
      if (prefixEl) prefixEl.value = '';
      const customPrefixEl = document.getElementById('epCustomPrefixInput');
      if (customPrefixEl) {
        customPrefixEl.value = '';
        customPrefixEl.classList.add('hidden');
      }
      document.getElementById('epPathInput').value = '';
      document.getElementById('epAuthRuleInput').value = 'public';
      updateRoleDropdownUI('epAuthRuleInput');
      document.getElementById('epTransactionInput').checked = false;
      document.getElementById('epSqlInput').value = `SELECT * FROM \`${currentCollection || 'users'}\` LIMIT :limit;`;
      document.getElementById('epDescInput').value = '';
      document.getElementById('epTestParamsInput').value = '{\n  "limit": 10\n}';
      updateCustomEpUrlPreview();
    }

    function selectCustomEndpoint(id) {
      const ep = customEndpoints.find(e => e.id === id);
      if (!ep) return;
      currentCustomEpId = ep.id;
      renderCustomEndpointsList();
      const alertEl = document.getElementById('customEpAlert');
      if (alertEl) alertEl.classList.add('hidden');

      document.getElementById('customEpFormTitle').textContent = `Edit Endpoint: ${ep.name || ep.path}`;
      document.getElementById('saveCustomEpBtnText').textContent = 'Save Changes';
      document.getElementById('deleteCustomEpBtn').classList.remove('hidden');

      document.getElementById('epNameInput').value = ep.name || '';
      document.getElementById('epMethodInput').value = ep.method || 'GET';

      // Parse path to automatically select prefix vs slug
      const p = (ep.path || '').trim().replace(/^\/+/, '');
      const prefixEl = document.getElementById('epPrefixSelect');
      const customPrefixEl = document.getElementById('epCustomPrefixInput');
      const pathEl = document.getElementById('epPathInput');

      if (p.startsWith('api/tables/')) {
        if (prefixEl) prefixEl.value = 'tables/';
        if (customPrefixEl) customPrefixEl.classList.add('hidden');
        if (pathEl) pathEl.value = p.substring('api/tables/'.length);
      } else if (p.startsWith('api/collections/')) {
        if (prefixEl) prefixEl.value = 'collections/';
        if (customPrefixEl) customPrefixEl.classList.add('hidden');
        if (pathEl) pathEl.value = p.substring('api/collections/'.length);
      } else if (p.startsWith('api/custom/')) {
        if (prefixEl) prefixEl.value = 'custom/';
        if (customPrefixEl) customPrefixEl.classList.add('hidden');
        if (pathEl) pathEl.value = p.substring('api/custom/'.length);
      } else if (p.startsWith('api/v1/')) {
        if (prefixEl) prefixEl.value = 'v1/';
        if (customPrefixEl) customPrefixEl.classList.add('hidden');
        if (pathEl) pathEl.value = p.substring('api/v1/'.length);
      } else if (p.startsWith('api/')) {
        if (prefixEl) prefixEl.value = '';
        if (customPrefixEl) customPrefixEl.classList.add('hidden');
        if (pathEl) pathEl.value = p.substring('api/'.length);
      } else {
        if (prefixEl) prefixEl.value = '/';
        if (customPrefixEl) customPrefixEl.classList.add('hidden');
        if (pathEl) pathEl.value = p;
      }

      document.getElementById('epAuthRuleInput').value = ep.authRule || ep.auth_rule || 'public';
      updateRoleDropdownUI('epAuthRuleInput');
      document.getElementById('epTransactionInput').checked = Boolean(ep.transaction);
      document.getElementById('epSqlInput').value = ep.sql || '';
      document.getElementById('epDescInput').value = ep.description || '';
      updateCustomEpUrlPreview();
    }

    // ================= VISUAL QUERY BUILDER =================
    let qbEditorMode = 'code'; // 'code' | 'visual'
    let qbCurrentAction = 'SELECT'; // 'SELECT' | 'INSERT' | 'UPDATE' | 'DELETE' | 'COUNT'
    let qbSelectedTable = '';
    let qbSelectedColumns = new Set();
    let qbFilterConditions = []; // Array of { logic: 'AND', col: '', op: '=', val: ':val' }

    function qbEscapeAttr(str) {
      if (str === null || str === undefined) return '';
      return String(str).replace(/"/g, '&quot;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
    }

    function switchEndpointEditorMode(mode) {
      qbEditorMode = mode;
      const codeBtn = document.getElementById('epModeCodeBtn');
      const visualBtn = document.getElementById('epModeVisualBtn');
      const codePane = document.getElementById('epSqlEditorPane');
      const visualPane = document.getElementById('epVisualBuilderPane');

      if (mode === 'visual') {
        if (codeBtn) {
          codeBtn.className = "px-3.5 py-1.5 rounded-lg text-xs font-bold transition flex items-center space-x-1.5 text-slate-500 hover:text-slate-800 cursor-pointer";
        }
        if (visualBtn) {
          visualBtn.className = "px-3.5 py-1.5 rounded-lg text-xs font-bold transition flex items-center space-x-1.5 bg-white shadow-2xs text-amber-600 font-extrabold cursor-pointer";
        }
        if (codePane) codePane.classList.add('hidden');
        if (visualPane) visualPane.classList.remove('hidden');

        initQueryBuilder();
      } else {
        if (codeBtn) {
          codeBtn.className = "px-3.5 py-1.5 rounded-lg text-xs font-bold transition flex items-center space-x-1.5 bg-white shadow-2xs text-emerald-600 font-extrabold cursor-pointer";
        }
        if (visualBtn) {
          visualBtn.className = "px-3.5 py-1.5 rounded-lg text-xs font-bold transition flex items-center space-x-1.5 text-slate-500 hover:text-slate-800 cursor-pointer";
        }
        if (codePane) codePane.classList.remove('hidden');
        if (visualPane) visualPane.classList.add('hidden');
      }
    }

    function initQueryBuilder() {
      populateQbTables();
      if (!qbSelectedTable && collections.length > 0) {
        qbSelectedTable = currentCollection || collections[0].name;
        const sel = document.getElementById('qbTableSelect');
        if (sel) sel.value = qbSelectedTable;
        onQbTableChange(qbSelectedTable, false);
      }
    }

    function populateQbTables() {
      const sel = document.getElementById('qbTableSelect');
      if (!sel) return;
      const current = sel.value || qbSelectedTable || currentCollection;
      sel.innerHTML = '<option value="">-- Choose a table --</option>' +
        collections.map(c => `<option value="${c.name}" ${c.name === current ? 'selected' : ''}>${c.name} (${c.type || 'base'})</option>`).join('');
    }

    function getTableFieldsList(tableName) {
      if (!tableName) return [];
      const col = collections.find(c => c.name === tableName);
      const cols = [{ name: 'id', type: 'text' }];
      if (col && col.type === 'auth') {
        cols.push(
          { name: 'email', type: 'email' },
          { name: 'username', type: 'text' },
          { name: 'role', type: 'select' },
          { name: 'verified', type: 'bool' }
        );
      }
      if (col && Array.isArray(col.schema)) {
        for (const f of col.schema) {
          if (!cols.some(c => c.name === f.name)) {
            cols.push({ name: f.name, type: f.type || 'text' });
          }
        }
      }
      if (!cols.some(c => c.name === 'created_at')) cols.push({ name: 'created_at', type: 'number' });
      if (!cols.some(c => c.name === 'updated_at')) cols.push({ name: 'updated_at', type: 'number' });
      return cols;
    }

    function onQbTableChange(tableName, shouldBuild = true) {
      qbSelectedTable = tableName;
      qbSelectedColumns.clear();

      const fields = getTableFieldsList(tableName);
      // Default: select all fields
      fields.forEach(f => qbSelectedColumns.add(f.name));

      renderQbColumns();
      populateQbOrderByOptions();

      // Clear or adapt filters to new table
      qbFilterConditions = [];
      renderQbFilters();

      if (shouldBuild) {
        buildSqlFromQb();
      }
    }

    function setQbAction(action) {
      qbCurrentAction = action;
      ['SELECT', 'INSERT', 'UPDATE', 'DELETE', 'COUNT'].forEach(act => {
        const btn = document.getElementById(`qbAction_${act}`);
        if (!btn) return;
        if (act === action) {
          btn.className = "py-1.5 px-2 rounded-lg text-center transition bg-emerald-600 text-white shadow-2xs font-extrabold";
        } else {
          btn.className = "py-1.5 px-2 rounded-lg text-center transition text-slate-600 hover:text-slate-900";
        }
      });

      const colsSec = document.getElementById('qbColumnsSection');
      const orderSec = document.getElementById('qbOrderLimitSection');
      const colsLabel = document.getElementById('qbColumnsLabel');

      if (colsSec) {
        if (action === 'DELETE' || action === 'COUNT') {
          colsSec.classList.add('hidden');
        } else {
          colsSec.classList.remove('hidden');
          if (action === 'SELECT') {
            if (colsLabel) colsLabel.textContent = 'Select Columns to Return';
          } else if (action === 'INSERT') {
            if (colsLabel) colsLabel.textContent = 'Columns to Insert (Values)';
          } else if (action === 'UPDATE') {
            if (colsLabel) colsLabel.textContent = 'Columns to Update (SET)';
          }
        }
      }

      if (orderSec) {
        if (action === 'SELECT') {
          orderSec.classList.remove('hidden');
        } else {
          orderSec.classList.add('hidden');
        }
      }

      // Sync HTTP method recommendation
      const methodEl = document.getElementById('epMethodInput');
      if (methodEl) {
        if (action === 'SELECT' || action === 'COUNT') methodEl.value = 'GET';
        else if (action === 'INSERT') methodEl.value = 'POST';
        else if (action === 'UPDATE') methodEl.value = 'PATCH';
        else if (action === 'DELETE') methodEl.value = 'DELETE';
      }

      buildSqlFromQb();
    }

    function renderQbColumns() {
      const container = document.getElementById('qbColumnsContainer');
      if (!container) return;

      const fields = getTableFieldsList(qbSelectedTable);
      if (fields.length === 0) {
        container.innerHTML = '<span class="text-xs text-slate-400 italic">No table selected or table has no columns.</span>';
        return;
      }

      container.innerHTML = fields.map(f => {
        const isSelected = qbSelectedColumns.has(f.name);
        return `
          <button type="button" onclick="qbToggleColumn('${f.name}')" class="px-2.5 py-1 rounded-lg text-xs font-mono font-semibold transition border flex items-center space-x-1.5 cursor-pointer ${
            isSelected 
              ? 'bg-emerald-50 border-emerald-400 text-emerald-800 font-bold shadow-2xs' 
              : 'bg-white border-slate-200 text-slate-500 hover:border-slate-300'
          }">
            <i class="fa-solid ${isSelected ? 'fa-check text-emerald-600' : 'fa-plus text-slate-300'} text-[10px]"></i>
            <span>${f.name}</span>
            <span class="text-[10px] text-slate-400 font-sans">(${f.type})</span>
          </button>
        `;
      }).join('');
    }

    function qbToggleColumn(colName) {
      if (qbSelectedColumns.has(colName)) {
        qbSelectedColumns.delete(colName);
      } else {
        qbSelectedColumns.add(colName);
      }
      renderQbColumns();
      buildSqlFromQb();
    }

    function qbSelectAllColumns(selectAll) {
      const fields = getTableFieldsList(qbSelectedTable);
      if (selectAll) {
        fields.forEach(f => qbSelectedColumns.add(f.name));
      } else {
        qbSelectedColumns.clear();
      }
      renderQbColumns();
      buildSqlFromQb();
    }

    function populateQbOrderByOptions() {
      const sel = document.getElementById('qbOrderByCol');
      if (!sel) return;
      const current = sel.value;
      const fields = getTableFieldsList(qbSelectedTable);
      sel.innerHTML = '<option value="">(None - Natural)</option>' +
        fields.map(f => `<option value="${f.name}" ${f.name === current ? 'selected' : ''}>${f.name}</option>`).join('');
    }

    function addQbFilterRow() {
      const fields = getTableFieldsList(qbSelectedTable);
      const defaultCol = fields.find(f => f.name === 'status')?.name || fields[0]?.name || 'id';
      qbFilterConditions.push({
        logic: qbFilterConditions.length === 0 ? 'WHERE' : 'AND',
        col: defaultCol,
        op: '=',
        val: `:${defaultCol}`
      });
      renderQbFilters();
      buildSqlFromQb();
    }

    function removeQbFilterRow(idx) {
      qbFilterConditions.splice(idx, 1);
      renderQbFilters();
      buildSqlFromQb();
    }

    function onQbFilterChange(idx, key, val) {
      if (qbFilterConditions[idx]) {
        qbFilterConditions[idx][key] = val;
        if (key === 'col' && (!qbFilterConditions[idx].val || qbFilterConditions[idx].val.startsWith(':'))) {
          qbFilterConditions[idx].val = `:${val}`;
        }
        renderQbFilters();
        buildSqlFromQb();
      }
    }

    function renderQbFilters() {
      const container = document.getElementById('qbFiltersContainer');
      if (!container) return;

      if (qbFilterConditions.length === 0) {
        container.innerHTML = '<div id="qbEmptyFiltersNotice" class="text-xs text-slate-400 italic py-1">No filter conditions. All matching rows will be affected.</div>';
        return;
      }

      const fields = getTableFieldsList(qbSelectedTable);

      container.innerHTML = qbFilterConditions.map((cond, idx) => {
        return `
          <div class="flex items-center space-x-2 bg-slate-50 p-2 rounded-xl border border-slate-200 text-xs">
            ${idx > 0 ? `
              <select onchange="onQbFilterChange(${idx}, 'logic', this.value)" class="bg-white border border-slate-300 rounded-lg px-2 py-1 font-bold text-indigo-700 text-xs font-mono">
                <option value="AND" ${cond.logic === 'AND' ? 'selected' : ''}>AND</option>
                <option value="OR" ${cond.logic === 'OR' ? 'selected' : ''}>OR</option>
              </select>
            ` : `
              <span class="px-2.5 py-1 text-slate-500 font-mono font-bold text-xs">WHERE</span>
            `}

            <!-- Column -->
            <select onchange="onQbFilterChange(${idx}, 'col', this.value)" class="bg-white border border-slate-300 rounded-lg px-2 py-1 font-mono text-xs text-slate-800">
              ${fields.map(f => `<option value="${f.name}" ${f.name === cond.col ? 'selected' : ''}>${f.name}</option>`).join('')}
            </select>

            <!-- Operator -->
            <select onchange="onQbFilterChange(${idx}, 'op', this.value)" class="bg-white border border-slate-300 rounded-lg px-2 py-1 font-mono font-bold text-xs text-slate-800">
              <option value="=" ${cond.op === '=' ? 'selected' : ''}>= (Equal)</option>
              <option value="!=" ${cond.op === '!=' ? 'selected' : ''}>!= (Not Equal)</option>
              <option value=">" ${cond.op === '>' ? 'selected' : ''}>&gt; (Greater)</option>
              <option value=">=" ${cond.op === '>=' ? 'selected' : ''}>&gt;= (Greater or Equal)</option>
              <option value="<" ${cond.op === '<' ? 'selected' : ''}>&lt; (Less)</option>
              <option value="<=" ${cond.op === '<=' ? 'selected' : ''}>&lt;= (Less or Equal)</option>
              <option value="LIKE" ${cond.op === 'LIKE' ? 'selected' : ''}>LIKE (%term%)</option>
              <option value="IN" ${cond.op === 'IN' ? 'selected' : ''}>IN (:list)</option>
              <option value="IS NULL" ${cond.op === 'IS NULL' ? 'selected' : ''}>IS NULL</option>
              <option value="IS NOT NULL" ${cond.op === 'IS NOT NULL' ? 'selected' : ''}>IS NOT NULL</option>
            </select>

            <!-- Value / Param -->
            ${cond.op !== 'IS NULL' && cond.op !== 'IS NOT NULL' ? `
              <input type="text" oninput="onQbFilterChange(${idx}, 'val', this.value)" value="${qbEscapeAttr(cond.val || '')}" placeholder=":${cond.col || 'param'}" class="flex-1 bg-white border border-slate-300 rounded-lg px-2.5 py-1 font-mono text-xs">
            ` : ''}

            <!-- Delete -->
            <button type="button" onclick="removeQbFilterRow(${idx})" class="p-1.5 text-slate-400 hover:text-rose-600 transition rounded-lg hover:bg-rose-50 cursor-pointer" title="Remove condition">
              <i class="fa-solid fa-trash-can text-xs"></i>
            </button>
          </div>
        `;
      }).join('');
    }

    function applyQbPlainText(message = 'hello') {
      const nameEl = document.getElementById('epNameInput');
      const pathEl = document.getElementById('epPathInput');
      const methodEl = document.getElementById('epMethodInput');
      const sqlEl = document.getElementById('epSqlInput');
      const paramsEl = document.getElementById('epTestParamsInput');

      if (nameEl && !nameEl.value) nameEl.value = 'Hello Message API';
      if (pathEl && !pathEl.value) pathEl.value = 'hello';
      if (methodEl) methodEl.value = 'GET';

      // Direct string response without SQL
      if (sqlEl) sqlEl.value = `'${message}'`;
      if (paramsEl) paramsEl.value = '{}';

      updateCustomEpUrlPreview();
      showToast(`Set direct plain text '${message}' (No SQL)`);
    }

    function buildSqlFromQb(force = false) {
      const autoSync = document.getElementById('qbAutoSyncCheckbox')?.checked;
      if (!autoSync && !force) return;

      const table = qbSelectedTable || 'users';
      const allFields = getTableFieldsList(table);
      let sql = '';
      const paramsObj = {};

      // Determine WHERE clause
      let whereClause = '';
      if (qbFilterConditions.length > 0) {
        const parts = qbFilterConditions.map((c, i) => {
          const prefix = i === 0 ? '' : ` ${c.logic} `;
          if (c.op === 'IS NULL' || c.op === 'IS NOT NULL') {
            return `${prefix}\`${c.col}\` ${c.op}`;
          } else if (c.op === 'LIKE') {
            const paramName = (c.val || `:${c.col}`).replace(/^:/, '');
            paramsObj[paramName] = "test";
            return `${prefix}\`${c.col}\` LIKE '%' || :${paramName} || '%'`;
          } else {
            const val = c.val || `:${c.col}`;
            if (val.startsWith(':')) {
              const pName = val.replace(/^:/, '');
              paramsObj[pName] = pName === 'id' ? "1" : (pName.includes('limit') ? 10 : "active");
            }
            return `${prefix}\`${c.col}\` ${c.op} ${val}`;
          }
        });
        whereClause = `\nWHERE ${parts.join('')}`;
      }

      if (qbCurrentAction === 'SELECT') {
        let colsList = '*';
        if (qbSelectedColumns.size > 0 && qbSelectedColumns.size < allFields.length) {
          colsList = Array.from(qbSelectedColumns).map(c => `\`${c}\``).join(', ');
        }

        const orderByCol = document.getElementById('qbOrderByCol')?.value;
        const orderDir = document.getElementById('qbOrderDir')?.value || 'DESC';
        let orderClause = '';
        if (orderByCol) {
          orderClause = `\nORDER BY \`${orderByCol}\` ${orderDir}`;
        }

        const limitVal = document.getElementById('qbLimitInput')?.value?.trim();
        const offsetVal = document.getElementById('qbOffsetInput')?.value?.trim();
        let limitClause = '';
        if (limitVal) {
          limitClause = `\nLIMIT ${limitVal}`;
          if (limitVal.startsWith(':')) paramsObj[limitVal.replace(/^:/, '')] = 10;
        }
        if (offsetVal) {
          limitClause += ` OFFSET ${offsetVal}`;
          if (offsetVal.startsWith(':')) paramsObj[offsetVal.replace(/^:/, '')] = 0;
        }

        sql = `SELECT ${colsList}\nFROM \`${table}\`${whereClause}${orderClause}${limitClause};`;

      } else if (qbCurrentAction === 'INSERT') {
        const targetCols = qbSelectedColumns.size > 0 
          ? Array.from(qbSelectedColumns).filter(c => c !== 'id' && c !== 'created_at' && c !== 'updated_at')
          : allFields.map(f => f.name).filter(c => c !== 'id' && c !== 'created_at' && c !== 'updated_at');

        const colNames = targetCols.map(c => `\`${c}\``).join(', ');
        const valNames = targetCols.map(c => `:${c}`).join(', ');

        targetCols.forEach(c => {
          const f = allFields.find(x => x.name === c);
          paramsObj[c] = f?.type === 'number' ? 100 : (f?.type === 'bool' ? true : `sample_${c}`);
        });

        sql = `INSERT INTO \`${table}\` (${colNames})\nVALUES (${valNames});`;

      } else if (qbCurrentAction === 'UPDATE') {
        const targetCols = qbSelectedColumns.size > 0 
          ? Array.from(qbSelectedColumns).filter(c => c !== 'id' && c !== 'created_at' && c !== 'updated_at')
          : allFields.map(f => f.name).filter(c => c !== 'id' && c !== 'created_at' && c !== 'updated_at');

        const setClauses = targetCols.map(c => `\`${c}\` = :${c}`).join(', ');

        targetCols.forEach(c => {
          const f = allFields.find(x => x.name === c);
          paramsObj[c] = f?.type === 'number' ? 200 : (f?.type === 'bool' ? false : `updated_${c}`);
        });

        const effectiveWhere = whereClause ? whereClause : '\nWHERE `id` = :id';
        if (!whereClause) paramsObj['id'] = "1";

        sql = `UPDATE \`${table}\`\nSET ${setClauses}${effectiveWhere};`;

      } else if (qbCurrentAction === 'DELETE') {
        const effectiveWhere = whereClause ? whereClause : '\nWHERE `id` = :id';
        if (!whereClause) paramsObj['id'] = "1";

        sql = `DELETE FROM \`${table}\`${effectiveWhere};`;

      } else if (qbCurrentAction === 'COUNT') {
        sql = `SELECT COUNT(*) AS total_count\nFROM \`${table}\`${whereClause} LIMIT 1;`;
      }

      const sqlEl = document.getElementById('epSqlInput');
      if (sqlEl) sqlEl.value = sql;

      const paramsEl = document.getElementById('epTestParamsInput');
      if (paramsEl && Object.keys(paramsObj).length > 0) {
        paramsEl.value = JSON.stringify(paramsObj, null, 2);
      }
    }

    function applySqlEndpointTemplate(kind) {
      if (!kind) return;
      const table = currentCollection || 'orders';
      const nameEl = document.getElementById('epNameInput');
      const sqlEl = document.getElementById('epSqlInput');
      const methodEl = document.getElementById('epMethodInput');
      const txEl = document.getElementById('epTransactionInput');
      const paramsEl = document.getElementById('epTestParamsInput');
      const pathEl = document.getElementById('epPathInput');

      if (!sqlEl) return;

      switch (kind) {
        case 'simple_text':
          if (nameEl && !nameEl.value) nameEl.value = 'Hello World Direct Text';
          if (pathEl && !pathEl.value) pathEl.value = 'hello';
          if (methodEl) methodEl.value = 'GET';
          if (txEl) txEl.checked = false;
          // Direct string response without SQL
          sqlEl.value = `'hello'`;
          if (paramsEl) {
            paramsEl.value = '{}';
          }
          break;

        case 'kasir_transaction':
          if (nameEl && !nameEl.value) nameEl.value = 'POS Cashier Checkout Transaction';
          if (pathEl && !pathEl.value) pathEl.value = 'pos/checkout';
          if (methodEl) methodEl.value = 'POST';
          if (txEl) txEl.checked = true;
          sqlEl.value = `-- ================================================================\n-- 🛒 POS / Cashier Checkout (Multi-Table Atomic Transaction)\n-- All queries below run inside a single database transaction.\n-- If any error occurs or stock is insufficient, all changes are ROLLED BACK.\n-- ================================================================\n\n-- 1. Insert into Header Table (orders / sales)\nINSERT INTO \`orders\` (\`id\`, \`customer_name\`, \`total_amount\`, \`payment_method\`, \`cashier_id\`, \`status\`)\nVALUES (:order_id, :customer_name, :total_amount, :payment_method, :cashier_id, 'completed');\n\n-- 2. Insert into Detail Table (order_items / sale_details)\nINSERT INTO \`order_items\` (\`order_id\`, \`product_id\`, \`product_name\`, \`quantity\`, \`price\`, \`subtotal\`)\nVALUES (:order_id, :product_id, :product_name, :quantity, :price, :subtotal);\n\n-- 3. Deduct product inventory stock in Products Table\nUPDATE \`products\`\nSET \`stock\` = \`stock\` - :quantity\nWHERE \`id\` = :product_id AND \`stock\` >= :quantity;`;
          if (paramsEl) {
            paramsEl.value = JSON.stringify({
              order_id: `ORD-${Date.now().toString().slice(-6)}`,
              customer_name: "Walk-in Customer",
              total_amount: 150.00,
              payment_method: "cash",
              cashier_id: "cashier_01",
              product_id: "PRD-001",
              product_name: "Premium Coffee Beans 1kg",
              quantity: 2,
              price: 75.00,
              subtotal: 150.00
            }, null, 2);
          }
          break;

        case 'multi_table_insert':
          if (nameEl && !nameEl.value) nameEl.value = 'Multi-Table Transaction (Table 1 + Table 2)';
          if (pathEl && !pathEl.value) pathEl.value = 'transactions/create';
          if (methodEl) methodEl.value = 'POST';
          if (txEl) txEl.checked = true;
          sqlEl.value = `-- ================================================================\n-- 📦 Multi-Table Transaction: Insert Table 1 & Table 2 Atomically\n-- ================================================================\n\n-- 1. Insert into Primary Table (Header / Parent Record)\nINSERT INTO \`transactions\` (\`id\`, \`reference_no\`, \`amount\`, \`notes\`)\nVALUES (:trx_id, :ref_no, :amount, :notes);\n\n-- 2. Insert into Secondary Table (Log / Detail / Audit Trail)\nINSERT INTO \`transaction_logs\` (\`transaction_id\`, \`action\`, \`user_id\`, \`details\`)\nVALUES (:trx_id, 'CREATED', :user_id, :details);`;
          if (paramsEl) {
            paramsEl.value = JSON.stringify({
              trx_id: `TRX-${Date.now().toString().slice(-6)}`,
              ref_no: `INV/${new Date().getFullYear()}/001`,
              amount: 250.00,
              notes: "Service Subscription Payment",
              user_id: "usr_admin",
              details: "Created via Custom SQL API"
            }, null, 2);
          }
          break;

        case 'dashboard_summary':
          if (nameEl && !nameEl.value) nameEl.value = 'Dashboard Metrics & Summary';
          if (pathEl && !pathEl.value) pathEl.value = 'dashboard/summary';
          if (methodEl) methodEl.value = 'GET';
          if (txEl) txEl.checked = false;
          sqlEl.value = `-- ================================================================\n-- 📊 Dashboard Metrics: Total Records, Status Counts & Revenue\n-- Gunakan LIMIT 1 jika ingin menghasilkan single JSON object { ... } langsung.\n-- ================================================================\nSELECT\n  COUNT(*) AS \`total_records\`,\n  COUNT(CASE WHEN \`status\` = 'active' OR \`status\` = 'completed' THEN 1 END) AS \`active_count\`,\n  COUNT(CASE WHEN \`status\` = 'pending' THEN 1 END) AS \`pending_count\`,\n  COALESCE(SUM(\`total_amount\`), 0) AS \`total_revenue\`,\n  COALESCE(AVG(\`total_amount\`), 0) AS \`average_amount\`,\n  MAX(\`created_at\`) AS \`latest_activity\`\nFROM \`${table}\` LIMIT 1;`;
          if (paramsEl) paramsEl.value = '{}';
          break;

        case 'dashboard_trends':
          if (nameEl && !nameEl.value) nameEl.value = 'Dashboard Status & Trends Breakdown';
          if (pathEl && !pathEl.value) pathEl.value = 'dashboard/trends';
          if (methodEl) methodEl.value = 'GET';
          if (txEl) txEl.checked = false;
          sqlEl.value = `-- ================================================================\n-- 📈 Dashboard Trends / Breakdown by Status or Category\n-- ================================================================\nSELECT\n  \`status\`,\n  COUNT(*) AS \`count\`,\n  COALESCE(SUM(\`total_amount\`), 0) AS \`total_value\`\nFROM \`${table}\`\nGROUP BY \`status\`\nORDER BY \`count\` DESC;`;
          if (paramsEl) paramsEl.value = '{}';
          break;

        case 'search_pagination':
          if (nameEl && !nameEl.value) nameEl.value = 'Search Records with Pagination';
          if (pathEl && !pathEl.value) pathEl.value = 'search/items';
          if (methodEl) methodEl.value = 'GET';
          if (txEl) txEl.checked = false;
          sqlEl.value = `-- ================================================================\n-- 🔍 Search & Filter Records with Limit & Offset Pagination\n-- ================================================================\nSELECT *\nFROM \`${table}\`\nWHERE (:search IS NULL OR :search = '' OR \`name\` LIKE '%' || :search || '%')\nORDER BY \`id\` DESC\nLIMIT :limit OFFSET :offset;`;
          if (paramsEl) {
            paramsEl.value = JSON.stringify({
              search: "",
              limit: 10,
              offset: 0
            }, null, 2);
          }
          break;

        case 'join_relation':
          if (nameEl && !nameEl.value) nameEl.value = 'Orders with Customer Details';
          if (pathEl && !pathEl.value) pathEl.value = 'reports/orders-joined';
          if (methodEl) methodEl.value = 'GET';
          if (txEl) txEl.checked = false;
          sqlEl.value = `-- ================================================================\n-- 🔗 JOIN Query: Combine Header Table with Related Customer Data\n-- ================================================================\nSELECT \n  o.\`id\` AS order_id,\n  o.\`total_amount\`,\n  o.\`status\`,\n  u.\`name\` AS customer_name,\n  u.\`email\` AS customer_email\nFROM \`orders\` o\nLEFT JOIN \`users\` u ON o.\`customer_id\` = u.\`id\`\nORDER BY o.\`id\` DESC\nLIMIT :limit;`;
          if (paramsEl) {
            paramsEl.value = JSON.stringify({
              limit: 20
            }, null, 2);
          }
          break;

        case 'upsert':
          if (nameEl && !nameEl.value) nameEl.value = 'Upsert Record (Insert or Update)';
          if (pathEl && !pathEl.value) pathEl.value = 'records/upsert';
          if (methodEl) methodEl.value = 'POST';
          if (txEl) txEl.checked = true;
          sqlEl.value = `-- ================================================================\n-- 🔄 Upsert: Insert if new, or Update if Primary Key exists\n-- ================================================================\nINSERT INTO \`${table}\` (\`id\`, \`name\`, \`status\`)\nVALUES (:id, :name, :status)\nON CONFLICT (\`id\`) DO UPDATE SET\n  \`name\` = excluded.\`name\`,\n  \`status\` = excluded.\`status\`;`;
          if (paramsEl) {
            paramsEl.value = JSON.stringify({
              id: "REC-001",
              name: "Sample Record",
              status: "active"
            }, null, 2);
          }
          break;

        case 'bulk_update':
          if (nameEl && !nameEl.value) nameEl.value = 'Bulk Update Status';
          if (pathEl && !pathEl.value) pathEl.value = 'records/update-status';
          if (methodEl) methodEl.value = 'POST';
          if (txEl) txEl.checked = true;
          sqlEl.value = `-- ================================================================\n-- ✏️ Update Record Status with Parameters\n-- ================================================================\nUPDATE \`${table}\`\nSET \`status\` = :status, \`updated_at\` = :updated_at\nWHERE \`id\` = :id;`;
          if (paramsEl) {
            paramsEl.value = JSON.stringify({
              id: "1",
              status: "approved",
              updated_at: Math.floor(Date.now() / 1000).toString()
            }, null, 2);
          }
          break;

        default:
          sqlEl.value = `SELECT * FROM \`${table}\` LIMIT :limit;`;
          if (paramsEl) paramsEl.value = '{\n  "limit": 10\n}';
          break;
      }

      updateCustomEpUrlPreview();
      showToast('SQL Template applied.');
    }

    function insertSqlToken(token) {
      const sqlEl = document.getElementById('epSqlInput');
      if (!sqlEl) return;
      const start = sqlEl.selectionStart || sqlEl.value.length;
      const end = sqlEl.selectionEnd || sqlEl.value.length;
      sqlEl.value = sqlEl.value.substring(0, start) + token + sqlEl.value.substring(end);
      sqlEl.focus();
    }

    async function saveCustomEndpointForm(silent) {
      const alertEl = document.getElementById('customEpAlert');
      if (alertEl) alertEl.classList.add('hidden');

      const name = document.getElementById('epNameInput').value.trim();
      const method = document.getElementById('epMethodInput').value;
      const path = getCombinedEndpointPath();
      const authRule = document.getElementById('epAuthRuleInput').value.trim() || 'public';
      const transaction = document.getElementById('epTransactionInput').checked;
      const sql = document.getElementById('epSqlInput').value.trim();
      const description = document.getElementById('epDescInput').value.trim();

      if (!path || !sql) {
        if (alertEl) {
          alertEl.textContent = 'URL Path Slug and SQL Query are required.';
          alertEl.classList.remove('hidden');
        }
        return null;
      }

      const payload = {
        id: currentCustomEpId || '',
        name: name || path,
        path,
        method,
        sql,
        description,
        authRule,
        transaction
      };

      const url = currentCustomEpId ? `/api/endpoints/${currentCustomEpId}` : '/api/endpoints';
      const httpMethod = currentCustomEpId ? 'PATCH' : 'POST';

      try {
        const res = await fetch(url, {
          method: httpMethod,
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify(payload)
        });
        const data = await res.json();
        if (!res.ok) {
          if (alertEl) {
            alertEl.textContent = data.message || 'Failed to save custom endpoint';
            alertEl.classList.remove('hidden');
          }
          return null;
        }
        const saved = data.item || data;
        currentCustomEpId = saved.id;
        if (!silent) {
          showToast(`Endpoint "${resolveEndpointFullPath(saved.path)}" saved!`);
        }
        await loadCustomEndpoints(saved.id);
        return saved;
      } catch (err) {
        if (alertEl) {
          alertEl.textContent = err.message;
          alertEl.classList.remove('hidden');
        }
        return null;
      }
    }

    async function deleteCurrentCustomEndpoint() {
      if (!currentCustomEpId) return;
      if (!confirm('Delete this custom SQL endpoint?')) return;
      const res = await fetch(`/api/endpoints/${currentCustomEpId}`, { method: 'DELETE' });
      if (res.ok) {
        currentCustomEpId = null;
        showToast('Custom endpoint deleted');
        await loadCustomEndpoints();
      } else {
        showToast('Failed to delete custom endpoint', 'error');
      }
    }

    async function runCustomEndpointLiveTest() {
      const saved = await saveCustomEndpointForm(true);
      if (!saved) return;

      const statusBadge = document.getElementById('epTestStatusBadge');
      const outPre = document.getElementById('epTestResponsePre');
      statusBadge.textContent = 'Running...';
      statusBadge.className = 'text-[10px] font-mono px-2 py-0.5 rounded bg-amber-100 text-amber-800';

      let paramsObj = {};
      const rawParams = (document.getElementById('epTestParamsInput')?.value || '').trim();
      if (rawParams) {
        try {
          paramsObj = JSON.parse(rawParams);
        } catch (e) {
          statusBadge.textContent = 'Invalid JSON Params';
          statusBadge.className = 'text-[10px] font-mono px-2 py-0.5 rounded bg-rose-100 text-rose-800';
          outPre.textContent = 'Error parsing Test Parameters JSON: ' + e.message;
          return;
        }
      }

      const execMethod = saved.method === 'ANY' ? 'POST' : saved.method;
      let targetUrl = resolveEndpointFullPath(saved.path);
      // Replace {param} or :param placeholders in route path using paramsObj if present
      Object.entries(paramsObj).forEach(([k, v]) => {
        if (targetUrl.includes(`{${k}}`)) {
          targetUrl = targetUrl.replace(`{${k}}`, encodeURIComponent(String(v)));
        } else if (targetUrl.includes(`:${k}`)) {
          targetUrl = targetUrl.replace(`:${k}`, encodeURIComponent(String(v)));
        }
      });
      const fetchOpts = {
        method: execMethod,
        headers: {}
      };

      if (execMethod === 'GET' || execMethod === 'DELETE') {
        const qs = new URLSearchParams();
        Object.entries(paramsObj).forEach(([k, v]) => {
          if (v !== undefined && v !== null) qs.append(k, String(v));
        });
        const qsStr = qs.toString();
        if (qsStr) targetUrl += `?${qsStr}`;
      } else {
        fetchOpts.headers['Content-Type'] = 'application/json';
        fetchOpts.body = JSON.stringify(paramsObj);
      }

      try {
        const t0 = performance.now();
        const res = await fetch(targetUrl, fetchOpts);
        const elapsed = (performance.now() - t0).toFixed(1);
        const resJson = await res.json();
        statusBadge.textContent = `${res.status} (${elapsed}ms)`;
        statusBadge.className = res.ok
          ? 'text-[10px] font-mono px-2 py-0.5 rounded bg-emerald-100 text-emerald-800 font-bold'
          : 'text-[10px] font-mono px-2 py-0.5 rounded bg-rose-100 text-rose-800 font-bold';
        outPre.textContent = JSON.stringify(resJson, null, 2);
      } catch (err) {
        statusBadge.textContent = 'Error';
        statusBadge.className = 'text-[10px] font-mono px-2 py-0.5 rounded bg-rose-100 text-rose-800 font-bold';
        outPre.textContent = err.message;
      }
    }

    function switchWorkspaceSubView(subView) {
      currentWorkspaceSubView = subView;
      if (subView === 'settings') {
        openSchemaEditorPage();
      } else if (subView === 'api') {
        openApiPreviewModal();
      } else if (subView === 'record') {
        openRecordDrawer(editingRecordId);
      } else if (subView === 'create-table') {
        openCreateTablePage();
      } else {
        closeRecordEditorPage();
        closeSchemaEditorPage();
        cancelCreateTablePage();
        closeApiPreviewModal();
      }
    }

    function switchSettingsTab(tab) {
      const tabs = ['quickmatrix', 'app', 'roles', 'backups', 'export', 'import', 'admins'];
      tabs.forEach(t => {
        const pane = document.getElementById(`settingsPane_${t}`);
        if (pane) pane.classList.toggle('hidden', t !== tab);
        const btn = document.getElementById(`setTab_${t}`);
        if (btn) {
          if (t === tab) {
            btn.className = "w-full text-left px-3 py-2 rounded-xl text-xs font-bold bg-emerald-600 text-white shadow-2xs flex items-center space-x-2.5 transition shrink-0";
          } else {
            btn.className = "w-full text-left px-3 py-2 rounded-xl text-xs font-semibold text-slate-700 hover:bg-slate-200/70 flex items-center space-x-2.5 transition shrink-0";
          }
        }
      });
      if (tab === 'quickmatrix') renderQuickMatrixTable();
      if (tab === 'roles') renderRolesSettings();
      if (tab === 'backups') loadBackups();
      if (tab === 'admins') loadAdminAccounts();
      if (tab === 'export') {
        document.getElementById('exportCollectionsPre').textContent = JSON.stringify(collections, null, 2);
      }
    }

    // ================= COLLECTIONS MANAGEMENT & 1-CLICK QUICK TOGGLES =================
    async function syncDatabaseTables() {
      const syncIcon = document.getElementById('syncIcon');
      if (syncIcon) syncIcon.classList.add('fa-spin');
      try {
        const res = await fetch('/api/collections/sync', { method: 'POST' });
        const data = await res.json();
        if (res.ok) {
          const list = data.item?.synced || data.synced || [];
          const count = list.length;
          if (count > 0) {
            showToast(`Successfully synced ${count} table(s): ${list.join(', ')}`);
          } else {
            showToast('All database tables are up to date');
          }
          await loadCollections();
          renderQuickMatrixTable();
        } else {
          showToast(data.message || 'Failed to sync database tables', 'error');
        }
      } catch (err) {
        showToast('Sync error: ' + err.message, 'error');
      } finally {
        if (syncIcon) syncIcon.classList.remove('fa-spin');
      }
    }

    async function loadCollections(targetCol) {
      try {
        const res = await fetch('/api/collections');
        const data = await res.json();
        if (data.debug?.database) {
          activeDbDriver = data.debug.database;
          const hb = document.getElementById('headerDbBadge');
          if (hb) hb.textContent = String(activeDbDriver).toUpperCase();
        }
        collections = data.item || data.items || [];
        const countBadge = document.getElementById('sidebarColCountBadge');
        if (countBadge) countBadge.textContent = collections.length;

        renderCollectionsList();

        const preferred = targetCol || localStorage.getItem('padi_active_col') || currentCollection;
        if (collections.some(c => c.name === preferred)) {
          currentCollection = preferred;
        } else if (collections.length > 0) {
          currentCollection = collections[0].name;
        }
        selectCollection(currentCollection);
      } catch (err) {
        console.error("Failed to load collections:", err);
      }
    }

    function describeAccessSummary(col) {
      if (!col) return 'Read: Public · Write: Auth Required';
      const isListPub = !col.listRule || col.listRule === 'public';
      const isCreatePub = col.createRule === 'public';
      const readLabel = isListPub ? 'Public' : 'Restricted';
      const writeLabel = isCreatePub ? 'Public' : 'Auth Required';
      return `Read: ${readLabel} · Write: ${writeLabel}`;
    }

    function renderCollectionsList() {
      const container = document.getElementById('collectionsList');
      const kw = (document.getElementById('collectionSearchInput')?.value || '').toLowerCase();
      const filtered = collections.filter(c => c.name.toLowerCase().includes(kw));

      container.innerHTML = filtered.map(c => {
        const isActive = c.name === currentCollection;
        const icon = c.type === 'auth' ? 'fa-user-shield' : 'fa-table';
        const isRealtime = c.realtime !== false;
        const isRel = c.autoLoadRelations === true || c.auto_load_relations === true;
        const fieldCount = (c.schema || []).length;
        return `
          <div class="group relative flex items-center rounded-xl transition border ${isActive ? 'bg-emerald-600 text-white border-emerald-600 shadow-sm font-bold' : 'bg-white hover:bg-emerald-50/70 text-slate-700 border-slate-100 font-semibold'}">
            <button onclick="selectCollection('${c.name}')" class="flex-1 flex items-center justify-between p-2.5 text-xs min-w-0">
              <div class="flex items-center space-x-2.5 truncate">
                <span class="w-7 h-7 rounded-lg flex items-center justify-center shrink-0 ${isActive ? 'bg-white/20 text-white' : 'bg-emerald-50 text-emerald-700'}">
                  <i class="fa-solid ${icon} text-[11px]"></i>
                </span>
                <div class="text-left truncate">
                  <div class="truncate font-mono">${c.name}</div>
                  <div class="text-[10px] ${isActive ? 'text-emerald-100' : 'text-slate-400'}">${c.type} &bull; ${fieldCount} columns</div>
                </div>
              </div>
              <div class="flex items-center space-x-1 shrink-0 ml-1">
                ${isRel ? `<span class="px-1.5 py-0.5 rounded-md text-[9px] font-mono ${isActive ? 'bg-white/20 text-white' : 'bg-teal-50 text-teal-700'}" title="Auto Relations Active"><i class="fa-solid fa-link"></i></span>` : ''}
                ${isRealtime
                  ? `<span class="px-1.5 py-0.5 rounded-md text-[9px] font-mono ${isActive ? 'bg-white/20 text-amber-200' : 'bg-emerald-50 text-emerald-700'}" title="Realtime SSE Active"><i class="fa-solid fa-bolt"></i></span>`
                  : `<span class="px-1.5 py-0.5 rounded-md text-[9px] font-mono ${isActive ? 'bg-white/10 text-emerald-200' : 'bg-slate-100 text-slate-400'}" title="Realtime Inactive">off</span>`}
              </div>
            </button>
            <button onclick="selectCollection('${c.name}'); switchWorkspaceSubView('settings');" class="p-2 mr-1 rounded-lg ${isActive ? 'text-emerald-100 hover:bg-white/15 hover:text-white' : 'text-slate-400 hover:bg-emerald-100 hover:text-emerald-800'} transition" title="Settings for table ${c.name}">
              <i class="fa-solid fa-gear text-[11px]"></i>
            </button>
          </div>
        `;
      }).join('');
    }

    function selectCollection(name) {
      if (window.innerWidth < 1024) {
        toggleMobileSidebar(false);
      }
      currentCollection = name;
      currentSort = "";
      localStorage.setItem('padi_active_col', name);
      selectedRecordIds.clear();
      updateBulkDeleteBar();
      renderCollectionsList();

      const col = collections.find(c => c.name === name);
      document.getElementById('currentCollectionTitle').textContent = name;
      document.getElementById('collectionTypeBadge').textContent = col ? col.type : 'base';

      const headerIcon = document.getElementById('currentColHeaderIcon');
      if (headerIcon) {
        headerIcon.className = col && col.type === 'auth' ? 'fa-solid fa-user-shield' : 'fa-solid fa-table-cells-large';
      }

      updateHeaderQuickBadges(col);

      if (currentWorkspaceSubView === 'settings') {
        renderInlineCollectionSettings();
      } else if (currentWorkspaceSubView === 'api') {
        renderApiPane();
      } else if (currentWorkspaceSubView === 'record') {
        if (!editingRecordId) {
          openRecordDrawer(null);
        } else {
          closeRecordEditorPage();
        }
      } else if (currentWorkspaceSubView === 'create-table') {
        cancelCreateTablePage();
      }

      currentPage = 1;
      loadRecords();
    }

    function updateHeaderQuickBadges(col) {
      const rtBadge = document.getElementById('collectionRealtimeBadge');
      if (rtBadge) {
        const isRt = !col || col.realtime !== false;
        rtBadge.title = `Realtime SSE: ${isRt ? 'ON' : 'OFF'} (Click to configure in Table Settings)`;
        if (isRt) {
          rtBadge.className = 'text-[11px] px-2.5 py-0.5 rounded-full bg-emerald-50 hover:bg-emerald-100 text-emerald-800 border border-emerald-300 font-bold inline-flex items-center space-x-1.5 transition cursor-pointer shadow-2xs';
          rtBadge.innerHTML = '<span class="w-2 h-2 rounded-full bg-emerald-500"></span><span>Realtime: <strong>ON</strong></span>';
        } else {
          rtBadge.className = 'text-[11px] px-2.5 py-0.5 rounded-full bg-slate-100 hover:bg-slate-200 text-slate-500 border border-slate-300 font-bold inline-flex items-center space-x-1.5 transition cursor-pointer shadow-2xs';
          rtBadge.innerHTML = '<span class="w-2 h-2 rounded-full bg-slate-400"></span><span>Realtime: <strong>OFF</strong></span>';
        }
      }

      const relBadge = document.getElementById('collectionRelationsBadge');
      if (relBadge) {
        const isAutoLoad = col && (col.autoLoadRelations === true || col.auto_load_relations === true);
        relBadge.title = `Auto Relations: ${isAutoLoad ? 'ON' : 'OFF'} (Click to configure in Table Settings)`;
        if (isAutoLoad) {
          relBadge.className = 'text-[11px] px-2.5 py-0.5 rounded-full bg-teal-50 hover:bg-teal-100 text-teal-800 border border-teal-300 font-bold inline-flex items-center space-x-1.5 transition cursor-pointer shadow-2xs';
          relBadge.innerHTML = '<i class="fa-solid fa-link text-[10px] text-teal-600"></i><span>Auto Relations: <strong>ON</strong></span>';
        } else {
          relBadge.className = 'text-[11px] px-2.5 py-0.5 rounded-full bg-slate-100 hover:bg-slate-200 text-slate-500 border border-slate-300 font-bold inline-flex items-center space-x-1.5 transition cursor-pointer shadow-2xs';
          relBadge.innerHTML = '<i class="fa-solid fa-link-slash text-[10px]"></i><span>Auto Relations: <strong>OFF</strong></span>';
        }
      }

      const blameBadge = document.getElementById('collectionBlameableBadge');
      if (blameBadge) {
        const isBlame = col && col.blameable === true;
        blameBadge.title = `Blameable Behaviour: ${isBlame ? 'ON' : 'OFF'} (Click to configure in Table Settings)`;
        if (isBlame) {
          blameBadge.className = 'text-[11px] px-2.5 py-0.5 rounded-full bg-indigo-50 hover:bg-indigo-100 text-indigo-800 border border-indigo-300 font-bold inline-flex items-center space-x-1.5 transition cursor-pointer shadow-2xs';
          blameBadge.innerHTML = '<i class="fa-solid fa-user-shield text-[10px] text-indigo-600"></i><span>Blameable: <strong>ON</strong></span>';
        } else {
          blameBadge.className = 'text-[11px] px-2.5 py-0.5 rounded-full bg-slate-100 hover:bg-slate-200 text-slate-500 border border-slate-300 font-bold inline-flex items-center space-x-1.5 transition cursor-pointer shadow-2xs';
          blameBadge.innerHTML = '<i class="fa-solid fa-user-slash text-[10px]"></i><span>Blameable: <strong>OFF</strong></span>';
        }
      }

      const ruleText = document.getElementById('collectionRuleSummaryText');
      if (ruleText) {
        ruleText.textContent = describeAccessSummary(col);
      }
    }

    // 1-Click Instant Toggle on Header or Matrix
    async function quickToggleCollectionSetting(fieldKey, targetColName) {
      const colName = targetColName || currentCollection;
      const col = collections.find(c => c.name === colName);
      if (!col) return;

      const updatedPayload = JSON.parse(JSON.stringify(col));
      if (fieldKey === 'realtime') {
        updatedPayload.realtime = !(col.realtime !== false);
      } else if (fieldKey === 'autoLoadRelations') {
        const curr = col.autoLoadRelations === true || col.auto_load_relations === true;
        updatedPayload.autoLoadRelations = !curr;
      } else if (fieldKey === 'blameable') {
        const curr = col.blameable === true;
        updatedPayload.blameable = !curr;
      }

      try {
        const res = await fetch(`/api/collections/${colName}`, {
          method: 'PATCH',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify(updatedPayload)
        });
        if (res.ok) {
          const label = fieldKey === 'realtime'
            ? `Realtime SSE (${colName}): ${updatedPayload.realtime ? 'ENABLED' : 'DISABLED'}`
            : (fieldKey === 'blameable'
              ? `Blameable Behaviour (${colName}): ${updatedPayload.blameable ? 'ENABLED' : 'DISABLED'}`
              : `Auto-Load Relations (${colName}): ${updatedPayload.autoLoadRelations ? 'ENABLED' : 'DISABLED'}`);
          showToast(label);
          await loadCollections(currentCollection);
          renderQuickMatrixTable();
        } else {
          showToast('Failed to save settings changes', 'error');
        }
      } catch (e) {
        showToast(e.message, 'error');
      }
    }

    // ================= INLINE COLLECTION SETTINGS & VISUAL ACCESS RULES =================
    function renderInlineCollectionSettings() {
      const col = collections.find(c => c.name === currentCollection) || collections[0];
      if (!col) return;

      // Always initialize inlineSchemaFields first
      inlineSchemaFields = col && Array.isArray(col.schema) ? JSON.parse(JSON.stringify(col.schema)) : [];

      const colNameEl = document.getElementById('inlineSetColName');
      if (colNameEl) colNameEl.textContent = col.name;

      const authBadge = document.getElementById('inlineColAuthSystemBadge');
      if (authBadge) authBadge.classList.toggle('hidden', col.type !== 'auth');

      const rtToggle = document.getElementById('inlineRealtimeToggle');
      if (rtToggle) rtToggle.checked = col.realtime !== false;

      const relToggle = document.getElementById('inlineAutoRelationsToggle');
      if (relToggle) relToggle.checked = col.autoLoadRelations === true || col.auto_load_relations === true;

      const blameToggle = document.getElementById('inlineBlameableToggle');
      if (blameToggle) blameToggle.checked = col.blameable === true;

      updateOptionsFieldNotices();

      const rulesConfig = [
        { key: 'listRule', label: '1. List Records (GET List)', desc: 'Who is allowed to list & search records?', defaultVal: 'public' },
        { key: 'viewRule', label: '2. View Single Record (GET Single)', desc: 'Who is allowed to view a single record detail?', defaultVal: 'public' },
        { key: 'createRule', label: '3. Create New Record (POST Create)', desc: 'Who is allowed to insert new records?', defaultVal: '@request.auth.id != ""' },
        { key: 'updateRule', label: '4. Update Record (PATCH Update)', desc: 'Who is allowed to edit existing records?', defaultVal: '@request.auth.id != ""' },
        { key: 'deleteRule', label: '5. Delete Record (DELETE)', desc: 'Who is allowed to delete records from the table?', defaultVal: '@request.auth.id != ""' }
      ];

      const ownerExpr = currentCollection === 'users' ? 'id = @request.auth.id' : 'user_id = @request.auth.id';

      const rulesGrid = document.getElementById('inlineRulesGrid');
      rulesGrid.innerHTML = rulesConfig.map((rc, i) => {
        const rawVal = (col[rc.key] && String(col[rc.key]).trim() !== '') ? col[rc.key] : rc.defaultVal;
        const inputId = `inlineInput_${rc.key}`;
        return `
          <div class="p-4 rounded-2xl bg-slate-50 border border-slate-200 space-y-2.5 ${i === 4 ? 'md:col-span-2' : ''}">
            <div class="flex flex-wrap items-center justify-between gap-1.5">
              <div>
                <div class="text-xs font-extrabold text-slate-800">${rc.label}</div>
                <div class="text-[11px] text-slate-500">${rc.desc}</div>
              </div>
              <div class="flex items-center flex-wrap gap-1 text-[10px]">
                <button type="button" onclick="applyRulePreset('${inputId}', 'public', 'set')" class="px-2 py-0.5 rounded-lg border border-slate-200 bg-white hover:bg-emerald-50 text-emerald-700 font-bold hover:border-emerald-300 transition" title="Public (Anyone allowed)">
                  Public
                </button>
                <button type="button" onclick="applyRulePreset('${inputId}', '@request.auth.id != \\'\\'', 'set')" class="px-2 py-0.5 rounded-lg border border-slate-200 bg-white hover:bg-slate-100 text-slate-600 font-semibold transition" title="Require Logged In User">
                  Require Auth
                </button>
                <button type="button" onclick="applyRulePreset('${inputId}', '${ownerExpr}', 'append')" class="px-2 py-0.5 rounded-lg border border-slate-200 bg-white hover:bg-indigo-50 text-indigo-700 font-bold hover:border-indigo-300 transition" title="Append Creator/Owner (${ownerExpr})">
                  Creator / Owner
                </button>

                <!-- Role Multi-Select Dropdown -->
                <div class="relative inline-block" id="roleDropdownWrapper_${inputId}">
                  <button type="button" onclick="toggleRoleDropdown('${inputId}')" id="roleBtn_${inputId}" class="px-2 py-0.5 rounded-lg border border-amber-200 bg-amber-50/70 hover:bg-amber-100 text-amber-800 font-bold hover:border-amber-300 transition flex items-center space-x-1" title="Select one or more authorized roles">
                    <i class="fa-solid fa-crown text-[9px] text-amber-600"></i>
                    <span id="roleBtnLabel_${inputId}">Role ▾</span>
                  </button>
                  <div id="roleDropdownMenu_${inputId}" class="hidden absolute right-0 top-7 w-56 bg-white border border-slate-200 rounded-2xl shadow-xl p-2.5 z-50 space-y-1">
                    <div class="px-2 py-1 text-[10px] font-extrabold uppercase text-slate-400 flex items-center justify-between">
                      <span>Authorized Roles</span>
                      <span class="text-[9px] text-slate-400 font-normal">Multi-select</span>
                    </div>
                    <div id="roleOptionsList_${inputId}" class="space-y-0.5 max-h-48 overflow-y-auto"></div>
                    <div class="pt-2 border-t border-slate-100 flex items-center space-x-1">
                      <input type="text" id="customRoleInput_${inputId}" placeholder="Custom role..." class="flex-1 bg-slate-50 border border-slate-200 rounded-lg px-2 py-1 text-[11px] font-mono focus:outline-none focus:border-amber-500" onkeydown="if(event.key==='Enter'){event.preventDefault();addCustomRoleToRule('${inputId}');}">
                      <button type="button" onclick="addCustomRoleToRule('${inputId}')" class="px-2.5 py-1 bg-amber-500 hover:bg-amber-600 text-white font-bold text-[10px] rounded-lg transition">
                        Add
                      </button>
                    </div>
                  </div>
                </div>

                <button type="button" onclick="applyRulePreset('${inputId}', '', 'set')" class="px-1.5 py-0.5 rounded-lg text-slate-400 hover:text-rose-600 transition" title="Clear expression">
                  <i class="fa-solid fa-xmark"></i>
                </button>
              </div>
            </div>
            <div>
              <input type="text" id="${inputId}" value="${rawVal.replace(/"/g, '&quot;')}" oninput="updateRoleDropdownUI('${inputId}')" class="w-full bg-white border border-slate-300 rounded-xl px-3.5 py-2 text-xs font-mono text-slate-800 focus:outline-none focus:border-emerald-600 shadow-2xs transition" placeholder='e.g. user_id = @request.auth.id || @request.auth.role = "admin"'>
            </div>
          </div>
        `;
      }).join('');

      // Initialize role dropdown labels and checks
      rulesConfig.forEach(rc => {
        updateRoleDropdownUI(`inlineInput_${rc.key}`);
      });

      // Initialize and render inline schema fields
      inlineSchemaFields = col && col.schema ? JSON.parse(JSON.stringify(col.schema)) : [];
      renderInlineDefaultFields(col);
      renderInlineSchemaFields();
    }

    function renderInlineDefaultFields(col) {
      const container = document.getElementById('inlineDefaultFieldsContainer');
      if (!container) return;
      const targetCol = col || collections.find(c => c.name === currentCollection) || {};
      const type = targetCol.type || 'base';
      const isBlameable = document.getElementById('inlineBlameableToggle')?.checked !== undefined
        ? document.getElementById('inlineBlameableToggle').checked
        : (targetCol.blameable === true);

      let fields = [];
      if (type === 'auth') {
        fields = [
          { name: 'id', type: 'text', sql: 'VARCHAR(255) PK', desc: 'Primary Key unik record user (Auto-generated)', icon: 'fa-solid fa-fingerprint', isBlameable: false },
          { name: 'email', type: 'email', sql: 'VARCHAR(255) UNIQUE', desc: 'Email kredensial login (Wajib unik & NOT NULL)', icon: 'fa-solid fa-envelope', isBlameable: false },
          { name: 'password', type: 'password', sql: 'VARCHAR(255) / TEXT', desc: 'Hash password terenkripsi bcrypt', icon: 'fa-solid fa-key', isBlameable: false },
          { name: 'name', type: 'text', sql: 'VARCHAR(255)', desc: 'Nama lengkap tampilan pengguna', icon: 'fa-solid fa-id-card', isBlameable: false },
          { name: 'username', type: 'text', sql: 'VARCHAR(50)', desc: 'Handle unik / username login pengguna', icon: 'fa-solid fa-at', isBlameable: false },
          { name: 'role', type: 'text', sql: 'VARCHAR(50)', desc: 'Role otorisasi akses (default: "user")', icon: 'fa-solid fa-shield-halved', isBlameable: false },
          { name: 'status', type: 'text', sql: 'VARCHAR(20)', desc: 'Status akun pengguna (default: "active")', icon: 'fa-solid fa-circle-check', isBlameable: false },
          { name: 'verified', type: 'bool', sql: 'INTEGER', desc: 'Status verifikasi email user (0 / 1)', icon: 'fa-solid fa-badge-check', isBlameable: false },
          { name: 'tokenKey', type: 'text', sql: 'VARCHAR(255)', desc: 'Kunci enkripsi token sesi & auth', icon: 'fa-solid fa-lock', isBlameable: false },
          { name: 'created_at', type: 'number', sql: 'BIGINT / TIMESTAMP', desc: 'Timestamp saat akun pertama dibuat', icon: 'fa-solid fa-calendar-plus', isBlameable: false },
          { name: 'updated_at', type: 'number', sql: 'BIGINT / TIMESTAMP', desc: 'Timestamp saat profil akun diubah', icon: 'fa-solid fa-clock-rotate-left', isBlameable: false },
          { 
            name: 'created_by', 
            type: 'text', 
            sql: 'VARCHAR(255)', 
            desc: isBlameable ? 'ID admin/user pembuat akun (Blameable aktif)' : 'ID admin/user pembuat akun (Nonaktif — klik badge untuk aktifkan di Table Options)', 
            icon: 'fa-solid fa-user-check', 
            isBlameable: true 
          },
          { 
            name: 'updated_by', 
            type: 'text', 
            sql: 'VARCHAR(255)', 
            desc: isBlameable ? 'ID admin/user pengubah akun (Blameable aktif)' : 'ID admin/user pengubah akun (Nonaktif — klik badge untuk aktifkan di Table Options)', 
            icon: 'fa-solid fa-user-pen', 
            isBlameable: true 
          }
        ];
      } else {
        // Base Table
        fields = [
          { name: 'id', type: 'text', sql: 'VARCHAR(255) PK', desc: 'Primary Key unik record (Auto-generated)', icon: 'fa-solid fa-fingerprint', isBlameable: false },
          { name: 'created_at', type: 'number', sql: 'BIGINT / TIMESTAMP', desc: 'Unix timestamp otomatis saat record di-insert', icon: 'fa-solid fa-calendar-plus', isBlameable: false },
          { name: 'updated_at', type: 'number', sql: 'BIGINT / TIMESTAMP', desc: 'Unix timestamp otomatis saat record di-update', icon: 'fa-solid fa-clock-rotate-left', isBlameable: false },
          { 
            name: 'created_by', 
            type: 'text', 
            sql: 'VARCHAR(255)', 
            desc: isBlameable ? 'ID user pembuat record (Blameable aktif, dilewati jika unauthenticated)' : 'ID user pembuat record (Nonaktif — klik badge untuk aktifkan di Table Options)', 
            icon: 'fa-solid fa-user-check', 
            isBlameable: true 
          },
          { 
            name: 'updated_by', 
            type: 'text', 
            sql: 'VARCHAR(255)', 
            desc: isBlameable ? 'ID user pengubah record (Blameable aktif, dilewati jika unauthenticated)' : 'ID user pengubah record (Nonaktif — klik badge untuk aktifkan di Table Options)', 
            icon: 'fa-solid fa-user-pen', 
            isBlameable: true 
          }
        ];
      }

      const countBadge = document.getElementById('inlineDefaultFieldsCountBadge');
      if (countBadge) countBadge.textContent = `${fields.length} default fields`;

      container.innerHTML = fields.map(f => {
        let badgeHtml = '';
        if (f.isBlameable) {
          if (isBlameable) {
            badgeHtml = `<span class="inline-flex items-center text-[10px] text-emerald-700 font-semibold px-2 py-0.5 rounded-full bg-emerald-50 border border-emerald-200 shrink-0 ml-1.5" title="Blameable behaviour aktif"><i class="fa-solid fa-circle-check text-[9px] mr-1 text-emerald-600"></i>Blameable: ON</span>`;
          } else {
            badgeHtml = `<button type="button" onclick="switchColSettingsTab('options')" class="inline-flex items-center text-[10px] text-slate-500 hover:text-indigo-700 font-semibold px-2 py-0.5 rounded-full bg-slate-100 hover:bg-indigo-50 border border-slate-200 hover:border-indigo-200 shrink-0 ml-1.5 transition cursor-pointer" title="Klik untuk mengaktifkan di tab Table Options"><i class="fa-solid fa-toggle-off text-[9px] mr-1 text-slate-400"></i>Blameable: OFF</button>`;
          }
        } else {
          badgeHtml = `<span class="inline-flex items-center text-[10px] text-emerald-700 font-semibold px-1.5 py-0.5 rounded bg-emerald-50 border border-emerald-200 shrink-0 ml-1.5" title="Field default ini dikelola otomatis oleh PadiBase"><i class="fa-solid fa-lock text-[8px] mr-1 text-emerald-600"></i>Default</span>`;
        }

        return `
        <div class="flex items-center justify-between p-2 rounded-xl bg-white border border-slate-200 shadow-2xs hover:border-slate-300 transition min-w-0 ${f.isBlameable && !isBlameable ? 'opacity-75' : ''}">
          <div class="flex items-center space-x-2.5 min-w-0">
            <div class="w-7 h-7 rounded-lg ${f.isBlameable && !isBlameable ? 'bg-slate-100 text-slate-500 border-slate-200' : 'bg-emerald-50 text-emerald-700 border-emerald-100'} flex items-center justify-center text-xs shrink-0 border">
              <i class="${f.icon}"></i>
            </div>
            <div class="min-w-0">
              <div class="flex items-center space-x-1.5 flex-wrap">
                <span class="font-mono font-bold text-slate-800 text-xs">${f.name}</span>
                <span class="text-[9px] font-mono px-1.5 py-0.2 rounded bg-slate-100 text-slate-600 border border-slate-200">${f.sql}</span>
              </div>
              <p class="text-[10px] text-slate-400 truncate mt-0.5" title="${f.desc}">${f.desc}</p>
            </div>
          </div>
          ${badgeHtml}
        </div>
        `;
      }).join('');
    }

    function renderInlineSchemaFields() {
      const col = collections.find(c => c.name === currentCollection);
      const isAuth = col && col.type === 'auth';
      const reserved = isAuth
        ? ['id', 'email', 'password', 'passwordhash', 'name', 'username', 'role', 'status', 'verified', 'tokenkey', 'created', 'updated', 'created_at', 'updated_at', 'created_by', 'updated_by']
        : ['id', 'created', 'updated', 'created_at', 'updated_at', 'created_by', 'updated_by'];

      // Filter user custom fields for the custom column architect
      const customItems = inlineSchemaFields
        .map((f, originalIdx) => ({ f, originalIdx }))
        .filter(item => !reserved.includes((item.f.name || '').toLowerCase().trim()));

      const countEl = document.getElementById('inlineColCountText');
      if (countEl) countEl.textContent = customItems.length;
      const colContainer = document.getElementById('inlineSchemaColumnsList');
      if (!colContainer) return;

      renderSchemaDataTypeButtons();

      if (customItems.length === 0) {
        colContainer.innerHTML = `<div class="py-6 text-center text-xs text-slate-400 border border-dashed border-slate-300 rounded-2xl">Belum ada kolom kustom tambahan. Gunakan tombol Add Column di bawah untuk menambahkan kolom data aplikasi Anda.</div>`;
        return;
      }

      const engineCfg = DB_ENGINE_TYPES[currentSchemaDbEngine] || DB_ENGINE_TYPES['sqlite'];

      colContainer.innerHTML = customItems.map(({ f, originalIdx: idx }) => {
        const isRel = f.type === 'relation';
        const targetTable = (f.options && (f.options.targetTable || f.options.collectionId)) || '';
        const targetField = (f.options && f.options.targetField) || 'id';
        const targetCol = collections.find(c => c.name === targetTable);
        const targetFieldsList = ['id'];
        if (targetCol && Array.isArray(targetCol.schema)) {
          targetCol.schema.forEach(sf => {
            if (sf.name && !targetFieldsList.includes(sf.name)) targetFieldsList.push(sf.name);
          });
        }
        const sqlType = getSqlTypeForField(f, currentSchemaDbEngine);
        const isVarchar = isVarcharType(f, currentSchemaDbEngine);
        const varcharLen = getVarcharLength(f);

        return `
        <div class="space-y-2 bg-slate-50 p-2.5 rounded-2xl border ${isRel ? 'border-teal-300 bg-teal-50/20 shadow-2xs' : 'border-slate-200'} text-xs transition min-w-0">
          ${isRel ? `
          <div class="flex flex-wrap items-center justify-between gap-1.5 bg-teal-50 border border-teal-200/90 rounded-xl px-2.5 py-1.5 text-[11px] text-teal-950 min-w-0">
            <div class="flex items-center space-x-1.5 flex-wrap gap-y-1 min-w-0">
              <span class="font-bold flex items-center space-x-1 text-teal-900 shrink-0">
                <i class="fa-solid fa-diagram-project text-teal-600"></i>
                <span>References:</span>
              </span>
              <div class="flex items-center space-x-1 min-w-0">
                <select onchange="updateInlineRelationTargetTable(${idx}, this.value)" class="bg-white border border-teal-300 rounded-lg px-2 py-1 font-mono text-[11px] font-bold text-teal-950 focus:outline-none focus:ring-1 focus:ring-teal-500 max-w-[150px] truncate">
                  <option value="">-- Target Table --</option>
                  ${collections.map(c => `<option value="${c.name}" ${targetTable === c.name ? 'selected' : ''}>${c.name}</option>`).join('')}
                </select>
                <span class="font-bold text-teal-700">.</span>
                <select onchange="updateInlineRelationTargetField(${idx}, this.value)" class="bg-white border border-teal-300 rounded-lg px-2 py-1 font-mono text-[11px] font-bold text-teal-950 focus:outline-none focus:ring-1 focus:ring-teal-500 max-w-[90px] truncate">
                  ${targetFieldsList.map(fl => `<option value="${fl}" ${targetField === fl ? 'selected' : ''}>${fl}</option>`).join('')}
                </select>
              </div>
            </div>
            <span class="hidden md:inline-flex items-center text-[10px] text-teal-800 font-medium bg-white/90 border border-teal-200/70 rounded-md px-2 py-0.5 shadow-2xs shrink-0">
              <i class="fa-solid fa-shield-halved text-teal-600 mr-1"></i>Auto Foreign Key
            </span>
          </div>
          ` : ''}
          <div class="flex items-center space-x-2 min-w-0 w-full">
            <div class="relative shrink-0">
              <select onchange="onInlineFieldTypeChange(${idx}, this.value)" class="appearance-none pl-6 pr-5 py-1.5 rounded-xl ${isRel ? 'bg-teal-100 hover:bg-teal-200 text-teal-900 border-teal-300' : 'bg-emerald-100 hover:bg-emerald-200 text-emerald-900 border-emerald-300'} font-mono font-bold text-[11px] border cursor-pointer focus:outline-none focus:ring-2 focus:ring-teal-500 transition max-w-[140px] truncate" title="Change column data type">
                ${engineCfg.types.map(t => {
                  const val = `${t.type}:${t.sqlType}`;
                  const isSelected = f.type === t.type && (!f.options?.sqlType || f.options.sqlType === t.sqlType);
                  const optLabel = (t.label.toUpperCase() === (t.sub || '').toUpperCase() || t.sub === 'FOREIGN KEY') ? t.label : `${t.label} (${t.sub})`;
                  return `<option value="${val}" ${isSelected ? 'selected' : ''}>${optLabel}</option>`;
                }).join('')}
              </select>
              <i class="${FIELD_ICONS[f.type] || 'fa-solid fa-font'} ${isRel ? 'text-teal-700' : 'text-emerald-700'} absolute left-2 top-1/2 -translate-y-1/2 pointer-events-none text-[11px]"></i>
              <i class="fa-solid fa-chevron-down ${isRel ? 'text-teal-700' : 'text-emerald-700'} absolute right-1.5 top-1/2 -translate-y-1/2 pointer-events-none text-[9px]"></i>
            </div>
            <input type="text" value="${f.name || ''}" oninput="inlineSchemaFields[${idx}].name = this.value.trim()" placeholder="column_name" class="flex-1 min-w-[70px] bg-white border border-slate-300 rounded-xl px-2.5 py-1.5 font-mono text-xs text-slate-900 focus:outline-none focus:border-teal-600">
            ${isVarchar ? `
            <div class="flex items-center space-x-1 shrink-0 bg-white border border-slate-300 rounded-xl px-1.5 py-1 shadow-2xs" title="Max character length (default: 255)">
              <span class="text-[10px] text-slate-400 font-semibold uppercase">Len:</span>
              <input type="number" min="1" max="65535" value="${varcharLen}" oninput="updateInlineVarcharLength(${idx}, this.value)" class="w-10 text-center font-mono font-bold text-xs text-slate-800 focus:outline-none" placeholder="255">
            </div>
            ` : ''}
            <span id="inlineSqlTypeBadge_${idx}" class="px-2 py-0.5 rounded-md font-mono text-[10px] bg-white text-slate-700 font-bold border border-slate-200 shrink-0 shadow-2xs whitespace-nowrap" title="Physical SQL type in ${engineCfg.name}">
              SQL: ${sqlType}
            </span>
            <label class="inline-flex items-center space-x-1 text-[11px] font-semibold text-slate-600 cursor-pointer shrink-0" title="Required column">
              <input type="checkbox" ${f.required ? 'checked' : ''} onchange="inlineSchemaFields[${idx}].required = this.checked" class="rounded border-slate-300 text-teal-600">
              <span>Req</span>
            </label>
            <label class="inline-flex items-center space-x-1 text-[11px] font-semibold text-slate-600 cursor-pointer shrink-0" title="Unique column">
              <input type="checkbox" ${f.unique ? 'checked' : ''} onchange="inlineSchemaFields[${idx}].unique = this.checked" class="rounded border-slate-300 text-teal-600">
              <span>Uniq</span>
            </label>
            <button type="button" onclick="removeInlineSchemaField(${idx})" class="w-7 h-7 rounded-xl hover:bg-rose-50 text-slate-400 hover:text-rose-600 flex items-center justify-center transition shrink-0" title="Delete column">
              <i class="fa-solid fa-trash-can"></i>
            </button>
          </div>
        </div>
        `;
      }).join('');
    }

    function onInlineFieldTypeChange(idx, val) {
      if (!inlineSchemaFields[idx]) return;
      if (val.includes(':')) {
        const [t, sql] = val.split(':');
        inlineSchemaFields[idx].type = t;
        inlineSchemaFields[idx].options = inlineSchemaFields[idx].options || {};
        if (sql.startsWith('VARCHAR')) {
          const len = getVarcharLength(inlineSchemaFields[idx]);
          inlineSchemaFields[idx].options.length = len;
          inlineSchemaFields[idx].options.sqlType = `VARCHAR(${len})`;
        } else {
          inlineSchemaFields[idx].options.sqlType = sql;
        }
      } else {
        inlineSchemaFields[idx].type = val;
      }
      if (inlineSchemaFields[idx].type === 'relation') {
        inlineSchemaFields[idx].options = inlineSchemaFields[idx].options || {};
        const defaultTarget = inlineSchemaFields[idx].options.targetTable || (collections.find(c => c.name !== currentCollection)?.name || collections[0]?.name || '');
        if (defaultTarget) {
          updateInlineRelationTargetTable(idx, defaultTarget);
          return;
        }
      }
      renderInlineSchemaFields();
    }

    function updateInlineRelationTargetTable(idx, targetTable) {
      if (!inlineSchemaFields[idx]) return;
      inlineSchemaFields[idx].options = inlineSchemaFields[idx].options || {};
      inlineSchemaFields[idx].options.targetTable = targetTable;
      inlineSchemaFields[idx].options.collectionId = targetTable;
      inlineSchemaFields[idx].options.targetField = 'id';

      // Always auto-update relation column name to match selected target table
      if (targetTable) {
        inlineSchemaFields[idx].name = formatRelationFieldName(targetTable, inlineSchemaFields, idx);
      }
      renderInlineSchemaFields();
    }

    function updateInlineRelationTargetField(idx, targetField) {
      if (!inlineSchemaFields[idx]) return;
      inlineSchemaFields[idx].options = inlineSchemaFields[idx].options || {};
      inlineSchemaFields[idx].options.targetField = targetField;
      renderInlineSchemaFields();
    }

    function addInlineSchemaField(type, sqlType) {
      const options = {};
      if (sqlType && sqlType.startsWith('VARCHAR')) {
        options.length = 255;
        options.sqlType = 'VARCHAR(255)';
      } else if (sqlType) {
        options.sqlType = sqlType;
      }
      let colName = `field_${inlineSchemaFields.length + 1}`;
      if (type === 'relation' && collections.length > 0) {
        const defaultTarget = collections.find(c => c.name !== currentCollection)?.name || collections[0].name;
        options.targetTable = defaultTarget;
        options.collectionId = defaultTarget;
        options.targetField = 'id';
        colName = formatRelationFieldName(defaultTarget, inlineSchemaFields);
      }
      inlineSchemaFields.push({
        name: colName,
        type,
        required: false,
        unique: false,
        options
      });
      renderInlineSchemaFields();
    }

    function removeInlineSchemaField(idx) {
      inlineSchemaFields.splice(idx, 1);
      renderInlineSchemaFields();
    }

    function openSchemaEditorPage(initialTab = 'fields') {
      const modal = document.getElementById('collectionSettingsModal');
      if (modal) modal.classList.remove('hidden');

      currentSchemaDbEngine = activeDbDriver || 'sqlite';
      try {
        renderInlineCollectionSettings();
        switchColSettingsTab(initialTab || 'fields');
        renderSchemaDataTypeButtons();
      } catch (err) {
        console.error('Error rendering collection settings:', err);
      }
    }

    function closeSchemaEditorPage() {
      const modal = document.getElementById('collectionSettingsModal');
      if (modal) modal.classList.add('hidden');
    }

    function switchColSettingsTab(tab) {
      ['fields', 'rules', 'options'].forEach(t => {
        const pane = document.getElementById(`colSettingsPane_${t}`);
        const btn = document.getElementById(`colTab_${t}`);
        if (pane) pane.classList.toggle('hidden', t !== tab);
        if (btn) {
          if (t === tab) {
            btn.className = "w-full text-left px-3 py-2.5 rounded-xl text-xs font-bold bg-emerald-600 text-white shadow-2xs flex items-center space-x-2.5 transition shrink-0";
          } else {
            btn.className = "w-full text-left px-3 py-2.5 rounded-xl text-xs font-semibold text-slate-700 hover:bg-slate-200/70 flex items-center space-x-2.5 transition shrink-0";
          }
        }
      });
    }

    function applyInlineSecurityProfile(profile) {
      const map = {
        recommended: {
          listRule: 'public',
          viewRule: 'public',
          createRule: '@request.auth.id != ""',
          updateRule: '@request.auth.id != ""',
          deleteRule: '@request.auth.id != ""'
        },
        public_all: {
          listRule: 'public',
          viewRule: 'public',
          createRule: 'public',
          updateRule: 'public',
          deleteRule: 'public'
        },
        private_all: {
          listRule: '@request.auth.id != ""',
          viewRule: '@request.auth.id != ""',
          createRule: '@request.auth.id != ""',
          updateRule: '@request.auth.id != ""',
          deleteRule: '@request.auth.id != ""'
        }
      }[profile];
      if (!map) return;
      Object.entries(map).forEach(([k, v]) => {
        const inputId = `inlineInput_${k}`;
        const inp = document.getElementById(inputId);
        if (inp) {
          inp.value = v;
          updateRoleDropdownUI(inputId);
        }
      });
      showToast('Access preset applied — click "Save Structure & Rules" to save');
    }

    function updateOptionsFieldNotices() {
      const col = collections.find(c => c.name === currentCollection) || {};
      const fields = inlineSchemaFields.length ? inlineSchemaFields : (col.schema || []);
      const hasCreatedBy = fields.some(f => f.name && f.name.toLowerCase() === 'created_by');
      const hasUpdatedBy = fields.some(f => f.name && f.name.toLowerCase() === 'updated_by');
      const hasCreatedAt = fields.some(f => f.name && f.name.toLowerCase() === 'created_at');
      const hasUpdatedAt = fields.some(f => f.name && f.name.toLowerCase() === 'updated_at');

      const isBlameOn = document.getElementById('inlineBlameableToggle')?.checked;
      const bNotice = document.getElementById('inlineBlameableNotice');
      if (bNotice) {
        if (!isBlameOn) {
          bNotice.innerHTML = `
            <div class="p-2.5 rounded-lg bg-slate-200/60 border border-slate-300/80 text-slate-600 text-xs flex items-center space-x-2">
              <i class="fa-solid fa-toggle-off text-slate-400"></i>
              <span>Blameable nonaktif. Field <code class="font-mono text-slate-700">created_by</code> dan <code class="font-mono text-slate-700">updated_by</code> tidak otomatis diisi.</span>
            </div>
          `;
        } else if (hasCreatedBy && hasUpdatedBy) {
          bNotice.innerHTML = `
            <div class="p-2.5 rounded-lg bg-emerald-500/10 border border-emerald-500/30 text-emerald-800 text-xs flex items-center space-x-2">
              <i class="fa-solid fa-circle-check text-emerald-600"></i>
              <span>Field <code class="font-mono font-bold bg-emerald-100/80 px-1 rounded">created_by</code> &amp; <code class="font-mono font-bold bg-emerald-100/80 px-1 rounded">updated_by</code> terdeteksi dan aktif.</span>
            </div>
          `;
        } else {
          const missing = [];
          if (!hasCreatedBy) missing.push('created_by');
          if (!hasUpdatedBy) missing.push('updated_by');
          bNotice.innerHTML = `
            <div class="p-2.5 rounded-lg bg-amber-500/10 border border-amber-500/30 text-amber-900 text-xs flex items-start justify-between gap-2 flex-wrap">
              <div class="flex items-start space-x-2">
                <i class="fa-solid fa-triangle-exclamation text-amber-500 mt-0.5 shrink-0"></i>
                <div>
                  <span class="font-bold">Notifikasi:</span> Field <code class="font-mono font-bold bg-amber-100/80 px-1 rounded">${missing.join(' & ')}</code> belum ada di skema tabel ini.
                  <p class="text-[11px] text-amber-800 mt-0.5">PadiBase akan otomatis membuatkannya saat Anda menyimpan pengaturan ini.</p>
                </div>
              </div>
              <button type="button" onclick="autoAddBlameableToSchema()" class="px-2.5 py-1 rounded-md bg-amber-600 hover:bg-amber-700 text-white text-[11px] font-bold transition flex items-center space-x-1 shrink-0">
                <i class="fa-solid fa-plus text-[10px]"></i>
                <span>Tambahkan ke Kolom Skema</span>
              </button>
            </div>
          `;
        }
      }

      const tNotice = document.getElementById('inlineTimestampsNotice');
      if (tNotice) {
        if (hasCreatedAt && hasUpdatedAt) {
          tNotice.innerHTML = `
            <div class="p-2.5 rounded-lg bg-emerald-500/10 border border-emerald-500/30 text-emerald-800 text-xs flex items-center space-x-2">
              <i class="fa-solid fa-circle-check text-emerald-600"></i>
              <span>Field timestamp <code class="font-mono font-bold bg-emerald-100/80 px-1 rounded">created_at</code> &amp; <code class="font-mono font-bold bg-emerald-100/80 px-1 rounded">updated_at</code> terdeteksi dan aktif.</span>
            </div>
          `;
        } else {
          const missingT = [];
          if (!hasCreatedAt) missingT.push('created_at');
          if (!hasUpdatedAt) missingT.push('updated_at');
          tNotice.innerHTML = `
            <div class="p-2.5 rounded-lg bg-sky-500/10 border border-sky-500/30 text-sky-900 text-xs flex items-start justify-between gap-2 flex-wrap">
              <div class="flex items-start space-x-2">
                <i class="fa-solid fa-clock text-sky-600 mt-0.5 shrink-0"></i>
                <div>
                  <span class="font-bold">Notifikasi:</span> Field <code class="font-mono font-bold bg-sky-100/80 px-1 rounded">${missingT.join(' & ')}</code> belum ada di tabel ini.
                  <p class="text-[11px] text-sky-800 mt-0.5">PadiBase otomatis mengisi waktu ini jika field tersedia di tabel.</p>
                </div>
              </div>
              <button type="button" onclick="autoAddTimestampsToSchema()" class="px-2.5 py-1 rounded-md bg-sky-600 hover:bg-sky-700 text-white text-[11px] font-bold transition flex items-center space-x-1 shrink-0">
                <i class="fa-solid fa-plus text-[10px]"></i>
                <span>Tambahkan ke Kolom Skema</span>
              </button>
            </div>
          `;
        }
      }
    }

    function autoAddBlameableToSchema() {
      let added = 0;
      if (!inlineSchemaFields.some(f => f.name && f.name.toLowerCase() === 'created_by')) {
        inlineSchemaFields.push({ name: 'created_by', type: 'text', required: false, unique: false, options: {} });
        added++;
      }
      if (!inlineSchemaFields.some(f => f.name && f.name.toLowerCase() === 'updated_by')) {
        inlineSchemaFields.push({ name: 'updated_by', type: 'text', required: false, unique: false, options: {} });
        added++;
      }
      renderInlineSchemaFields();
      updateOptionsFieldNotices();
      showToast(`Berhasil menambahkan ${added} kolom blameable ke skema tabel.`);
    }

    function autoAddTimestampsToSchema() {
      let added = 0;
      if (!inlineSchemaFields.some(f => f.name && f.name.toLowerCase() === 'created_at')) {
        inlineSchemaFields.push({ name: 'created_at', type: 'text', required: false, unique: false, options: {} });
        added++;
      }
      if (!inlineSchemaFields.some(f => f.name && f.name.toLowerCase() === 'updated_at')) {
        inlineSchemaFields.push({ name: 'updated_at', type: 'text', required: false, unique: false, options: {} });
        added++;
      }
      renderInlineSchemaFields();
      updateOptionsFieldNotices();
      showToast(`Berhasil menambahkan ${added} kolom timestamp (created_at & updated_at) ke skema tabel.`);
    }

    function onInlineBlameableToggleChange(checked) {
      updateOptionsFieldNotices();
      renderInlineDefaultFields();
      if (checked) {
        const hasCreatedBy = inlineSchemaFields.some(f => f.name && f.name.toLowerCase() === 'created_by');
        const hasUpdatedBy = inlineSchemaFields.some(f => f.name && f.name.toLowerCase() === 'updated_by');
        if (!hasCreatedBy || !hasUpdatedBy) {
          showToast('Blameable diaktifkan. Kolom created_by & updated_by akan dipastikan dibuat saat disimpan.', 'info');
        } else {
          showToast('Blameable aktif.', 'success');
        }
      } else {
        showToast('Blameable dinonaktifkan.', 'info');
      }
    }

    async function saveInlineCollectionSettings() {
      const col = collections.find(c => c.name === currentCollection);
      if (!col) return;

      const cleanSchema = inlineSchemaFields.filter(f => f.name && f.name.trim() !== '');

      const isBlameable = document.getElementById('inlineBlameableToggle')?.checked === true;
      if (isBlameable) {
        if (!cleanSchema.some(f => f.name && f.name.toLowerCase() === 'created_by')) {
          cleanSchema.push({ name: 'created_by', type: 'text', required: false, unique: false, options: {} });
        }
        if (!cleanSchema.some(f => f.name && f.name.toLowerCase() === 'updated_by')) {
          cleanSchema.push({ name: 'updated_by', type: 'text', required: false, unique: false, options: {} });
        }
      }

      const payload = {
        name: col.name,
        type: col.type,
        schema: cleanSchema,
        realtime: document.getElementById('inlineRealtimeToggle').checked,
        autoLoadRelations: document.getElementById('inlineAutoRelationsToggle').checked,
        blameable: isBlameable,
        listRule: document.getElementById('inlineInput_listRule')?.value.trim() || 'public',
        viewRule: document.getElementById('inlineInput_viewRule')?.value.trim() || 'public',
        createRule: document.getElementById('inlineInput_createRule')?.value.trim() || '@request.auth.id != ""',
        updateRule: document.getElementById('inlineInput_updateRule')?.value.trim() || '@request.auth.id != ""',
        deleteRule: document.getElementById('inlineInput_deleteRule')?.value.trim() || '@request.auth.id != ""'
      };

      try {
        const res = await fetch(`/api/collections/${currentCollection}`, {
          method: 'PATCH',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify(payload)
        });
        if (res.ok) {
          closeSchemaEditorPage();
          showToast(`Table "${currentCollection}" structure & rules saved successfully!`);
          await loadCollections(currentCollection);
        } else {
          const d = await res.json();
          showToast(d.message || 'Failed to save settings', 'error');
        }
      } catch (e) {
        showToast(e.message, 'error');
      }
    }

    // ================= GLOBAL QUICK SETTINGS MATRIX (SYSTEM & CONFIG VIEW) =================
    function renderQuickMatrixTable() {
      const tbody = document.getElementById('quickMatrixTableBody');
      if (!tbody) return;
      if (collections.length === 0) {
        tbody.innerHTML = `<tr><td colspan="6" class="py-8 text-center text-slate-400">No tables registered yet.</td></tr>`;
        return;
      }

      tbody.innerHTML = collections.map(c => {
        const isRt = c.realtime !== false;
        const isRel = c.autoLoadRelations === true || c.auto_load_relations === true;
        const isPubWrite = c.createRule === 'public' && c.updateRule === 'public';
        const isPrivRead = c.listRule && c.listRule !== 'public';
        const secMode = isPubWrite ? 'public_all' : (isPrivRead ? 'private_all' : 'recommended');

        return `
          <tr class="hover:bg-emerald-50/30 transition">
            <td class="py-3.5 px-4">
              <div class="flex items-center space-x-2.5">
                <span class="w-8 h-8 rounded-xl bg-emerald-50 text-emerald-700 flex items-center justify-center font-bold">
                  <i class="fa-solid ${c.type === 'auth' ? 'fa-user-shield' : 'fa-table'}"></i>
                </span>
                <div>
                  <div class="font-mono font-extrabold text-slate-900">${c.name}</div>
                  <div class="text-[10px] text-slate-400 font-mono">ID: ${c.id}</div>
                </div>
              </div>
            </td>
            <td class="py-3.5 px-4">
              <span class="px-2 py-0.5 rounded-md bg-slate-100 text-slate-700 font-mono text-[11px] font-bold uppercase">${c.type}</span>
              <span class="text-slate-400 ml-1.5 text-[11px]">${(c.schema || []).length} columns</span>
            </td>
            <td class="py-3.5 px-4">
              <button onclick="quickToggleCollectionSetting('realtime', '${c.name}')" class="px-3 py-1 rounded-full text-xs font-bold border transition ${isRt ? 'bg-emerald-50 text-emerald-800 border-emerald-300 hover:bg-emerald-100' : 'bg-slate-100 text-slate-500 border-slate-300 hover:bg-slate-200'}">
                <i class="fa-solid fa-bolt mr-1 ${isRt ? 'text-emerald-600' : 'text-slate-400'}"></i>
                ${isRt ? 'ON (Active)' : 'OFF'}
              </button>
            </td>
            <td class="py-3.5 px-4">
              <button onclick="quickToggleCollectionSetting('autoLoadRelations', '${c.name}')" class="px-3 py-1 rounded-full text-xs font-bold border transition ${isRel ? 'bg-teal-50 text-teal-800 border-teal-300 hover:bg-teal-100' : 'bg-slate-100 text-slate-500 border-slate-300 hover:bg-slate-200'}">
                <i class="fa-solid fa-link mr-1 ${isRel ? 'text-teal-600' : 'text-slate-400'}"></i>
                ${isRel ? 'ON (Auto)' : 'OFF'}
              </button>
            </td>
            <td class="py-3.5 px-4">
              <select onchange="quickApplyMatrixRule('${c.name}', this.value)" class="bg-slate-50 border border-slate-300 rounded-xl px-2.5 py-1.5 text-xs font-semibold text-slate-800 focus:outline-none focus:border-emerald-600">
                <option value="recommended" ${secMode === 'recommended' ? 'selected' : ''}>🌐 Public Read · Auth Write</option>
                <option value="public_all" ${secMode === 'public_all' ? 'selected' : ''}>🔓 Full Public (All Open)</option>
                <option value="private_all" ${secMode === 'private_all' ? 'selected' : ''}>🔐 Require Auth</option>
              </select>
            </td>
            <td class="py-3.5 px-4 text-right space-x-1.5 whitespace-nowrap">
              <button onclick="switchMainView('collections'); selectCollection('${c.name}'); switchWorkspaceSubView('records');" class="px-2.5 py-1.5 rounded-lg border border-slate-200 hover:bg-slate-100 text-slate-700 font-bold text-[11px]">
                Open Data
              </button>
              <button onclick="switchMainView('collections'); selectCollection('${c.name}'); openCollectionDrawer(true);" class="px-2.5 py-1.5 rounded-lg bg-emerald-50 hover:bg-emerald-100 text-emerald-800 border border-emerald-200 font-bold text-[11px]">
                <i class="fa-solid fa-sliders mr-1"></i> Schema &amp; Rules
              </button>
            </td>
          </tr>
        `;
      }).join('');
    }

    async function quickApplyMatrixRule(colName, profile) {
      const col = collections.find(c => c.name === colName);
      if (!col) return;
      const updated = JSON.parse(JSON.stringify(col));
      if (profile === 'public_all') {
        updated.listRule = 'public';
        updated.viewRule = 'public';
        updated.createRule = 'public';
        updated.updateRule = 'public';
        updated.deleteRule = 'public';
      } else if (profile === 'private_all') {
        updated.listRule = '@request.auth.id != ""';
        updated.viewRule = '@request.auth.id != ""';
        updated.createRule = '@request.auth.id != ""';
        updated.updateRule = '@request.auth.id != ""';
        updated.deleteRule = '@request.auth.id != ""';
      } else {
        updated.listRule = 'public';
        updated.viewRule = 'public';
        updated.createRule = '@request.auth.id != ""';
        updated.updateRule = '@request.auth.id != ""';
        updated.deleteRule = '@request.auth.id != ""';
      }

      const res = await fetch(`/api/collections/${colName}`, {
        method: 'PATCH',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(updated)
      });
      if (res.ok) {
        showToast(`Access rules for table "${colName}" updated successfully!`);
        await loadCollections(currentCollection);
        renderQuickMatrixTable();
      }
    }

    // ================= RECORDS TABLE, VISUAL FILTER BUILDER & SEARCH =================
    function toggleVisualFilterBuilder() {
      const el = document.getElementById('visualFilterHelper');
      if (!el) return;
      el.classList.toggle('hidden');
      if (!el.classList.contains('hidden')) {
        populateVisualFilterColumns();
      }
    }

    function populateVisualFilterColumns() {
      const col = collections.find(c => c.name === currentCollection);
      const cols = getCollectionColumns(col, currentRecords);
      const sel = document.getElementById('vfColSelect');
      if (!sel) return;
      const allCols = [{ name: 'id' }, ...cols];
      sel.innerHTML = allCols.map(c => `<option value="${c.name}">${c.name}</option>`).join('');
    }

    function applyVisualFilterCondition() {
      const colName = document.getElementById('vfColSelect')?.value || 'id';
      const op = document.getElementById('vfOpSelect')?.value || '~';
      const rawVal = (document.getElementById('vfValInput')?.value || '').trim();
      if (!rawVal) {
        clearFilterSearch();
        return;
      }
      const isNum = !isNaN(Number(rawVal)) && rawVal !== '' && (op === '>' || op === '<' || op === '=');
      const formattedVal = isNum ? rawVal : `"${rawVal.replace(/"/g, '')}"`;
      const expr = `${colName} ${op} ${formattedVal}`;
      document.getElementById('recordFilterInput').value = expr;
      applyFilterSearch();
    }

    function applyFilterSearch() {
      currentFilter = document.getElementById('recordFilterInput').value.trim();
      document.getElementById('clearFilterBtn').classList.toggle('hidden', !currentFilter);
      currentPage = 1;
      loadRecords();
    }

    function clearFilterSearch() {
      document.getElementById('recordFilterInput').value = '';
      const vfVal = document.getElementById('vfValInput');
      if (vfVal) vfVal.value = '';
      currentFilter = '';
      document.getElementById('clearFilterBtn').classList.add('hidden');
      currentPage = 1;
      loadRecords();
    }

    function toggleSort(colName) {
      if (currentSort === colName) {
        currentSort = `-${colName}`;
      } else {
        currentSort = colName;
      }
      loadRecords();
    }

    function changePage(delta) {
      currentPage = Math.max(1, currentPage + delta);
      loadRecords();
    }

    async function loadRecords() {
      try {
        const params = new URLSearchParams({
          page: currentPage,
          perPage: 30,
        });
        if (currentSort) params.set('sort', currentSort);
        if (currentFilter) params.set('filter', currentFilter);

        const res = await fetch(`/api/collections/${currentCollection}/records?${params.toString()}`);
        const data = await res.json();
        if (!res.ok) {
          showToast(data.message || 'Filter query error', 'error');
          return;
        }
        currentRecords = data.item || data.items || [];
        populateVisualFilterColumns();
        renderRecordsTable(data);
      } catch (err) {
        console.error("Failed to load records:", err);
      }
    }

    function getCollectionColumns(col, items) {
      const cols = [];
      if (col && col.type === 'auth') {
        cols.push({ name: 'name', type: 'text' });
        cols.push({ name: 'username', type: 'text' });
        cols.push({ name: 'email', type: 'email' });
        cols.push({ name: 'role', type: 'text' });
        cols.push({ name: 'status', type: 'text' });
        cols.push({ name: 'verified', type: 'bool' });
      }
      if (col && col.schema) {
        for (const f of col.schema) {
          if (!cols.some(c => c.name === f.name)) {
            cols.push({ name: f.name, type: f.type });
          }
        }
      }
      for (const item of items) {
        for (const k of Object.keys(item)) {
          if (!k.startsWith('@') && !['id', 'created', 'updated', 'collectionId', 'collectionName', 'emailVisibility'].includes(k)) {
            if (!cols.some(c => c.name === k)) {
              cols.push({ name: k, type: 'text' });
            }
          }
        }
      }
      return cols;
    }

    function renderRecordsTable(data) {
      const tbody = document.getElementById('recordsBody');
      const thead = document.getElementById('tableHeader');
      const items = data.item || data.items || [];
      const totalItems = data.meta?.total ?? data.totalItems ?? 0;
      const pageNum = data.meta?.current_page ?? data.page ?? 1;
      const totalPages = data.meta?.last_page ?? data.totalPages ?? 1;
      const col = collections.find(c => c.name === currentCollection);
      const schemaCols = getCollectionColumns(col, items);

      const totalColCount = 2 + schemaCols.length + 1;

      const sortIcon = (field) => {
        if (currentSort === field) return '<i class="fa-solid fa-caret-up ml-1 text-emerald-600"></i>';
        if (currentSort === `-${field}`) return '<i class="fa-solid fa-caret-down ml-1 text-emerald-600"></i>';
        return '';
      };

      thead.innerHTML = `
        <th class="py-3 px-4 w-8 border-r border-slate-200/60">
          <input type="checkbox" id="selectAllCheckbox" onchange="toggleSelectAll(this.checked)" class="rounded border-slate-300">
        </th>
        <th onclick="toggleSort('id')" class="py-3 px-4 border-r border-slate-200/60 cursor-pointer hover:bg-emerald-50/50 transition whitespace-nowrap">
          <i class="fa-solid fa-fingerprint text-[10px] text-emerald-600 mr-1"></i> id ${sortIcon('id')}
        </th>
        ${schemaCols.map(c => `
          <th onclick="toggleSort('${c.name}')" class="py-3 px-4 border-r border-slate-200/60 cursor-pointer hover:bg-emerald-50/50 transition whitespace-nowrap">
            <i class="${FIELD_ICONS[c.type] || 'fa-solid fa-font'} text-[10px] text-emerald-600 mr-1"></i> ${c.name} ${sortIcon(c.name)}
          </th>
        `).join('')}
        <th class="py-3 px-4 w-10"></th>
      `;

      if (items.length === 0) {
        tbody.innerHTML = `
          <tr>
            <td colspan="${totalColCount}" class="py-16 text-center text-slate-400 font-sans">
              <div class="w-12 h-12 rounded-2xl bg-emerald-50 text-emerald-600 flex items-center justify-center mx-auto mb-3 text-lg">
                <i class="fa-solid fa-seedling"></i>
              </div>
              <div class="text-sm font-bold text-slate-700 mb-1">No records found in table "${currentCollection}"</div>
              <p class="text-xs text-slate-400 mb-4">Add your first record or configure the table columns.</p>
              <div class="flex items-center justify-center space-x-2">
                <button onclick="openCollectionDrawer(true)" class="px-3.5 py-2 rounded-xl border border-slate-300 hover:bg-slate-100 text-slate-700 text-xs font-bold">
                  <i class="fa-solid fa-sliders mr-1"></i> Configure Schema &amp; Rules
                </button>
                <button onclick="openRecordDrawer(null)" class="px-4 py-2 rounded-xl padi-btn-primary text-white text-xs font-bold shadow-sm">
                  + Add First Record
                </button>
              </div>
            </td>
          </tr>
        `;
      } else {
        tbody.innerHTML = items.map(item => {
          const checked = selectedRecordIds.has(item.id) ? 'checked' : '';
          return `
            <tr onclick="openRecordDrawer('${item.id}')" class="hover:bg-emerald-50/40 cursor-pointer transition group">
              <td onclick="event.stopPropagation()" class="py-2.5 px-4 border-r border-slate-100">
                <input type="checkbox" ${checked} onchange="toggleSelectRecord('${item.id}', this.checked)" class="rounded border-slate-300">
              </td>
              <td onclick="event.stopPropagation(); copyToClipboard('${item.id}')" class="py-2.5 px-4 border-r border-slate-100 whitespace-nowrap">
                <span class="px-2.5 py-0.5 rounded-lg bg-emerald-50 group-hover:bg-emerald-100 text-emerald-800 border border-emerald-200/70 font-semibold text-[11px] inline-flex items-center">
                  ${item.id} <i class="fa-regular fa-copy ml-1.5 text-[9px] opacity-50"></i>
                </span>
              </td>
              ${schemaCols.map(c => `<td class="py-2.5 px-4 border-r border-slate-100 max-w-[240px] truncate">${formatCell(item[c.name], c.type, item)}</td>`).join('')}
              <td class="py-2.5 px-4 text-right text-slate-400 group-hover:text-emerald-700">
                <i class="fa-solid fa-pen-to-square text-xs"></i>
              </td>
            </tr>
          `;
        }).join('');
      }

      document.getElementById('paginationInfo').textContent = `Showing ${items.length} of ${totalItems} records`;
      document.getElementById('currentPageBadge').textContent = `${pageNum} / ${totalPages}`;
      document.getElementById('prevPageBtn').disabled = pageNum <= 1;
      document.getElementById('nextPageBtn').disabled = pageNum >= totalPages;
    }

    function formatCell(val, type, item) {
      if (val === null || val === undefined || val === '') return '<span class="text-slate-300">—</span>';
      if (type === 'file' || (typeof val === 'string' && /\.(jpg|jpeg|png|gif|webp|svg|pdf|docx|xlsx|zip|mp4|mp3|txt)$/i.test(val))) {
        const isImg = /\.(jpg|jpeg|png|gif|webp|svg)$/i.test(String(val));
        const recId = item ? item.id : '';
        const url = `/api/files/${currentCollection}/${recId}/${val}`;
        if (isImg && item) {
          return `
            <div onclick="event.stopPropagation(); openImageLightbox('${url}', '${val}')" class="inline-flex items-center space-x-2 text-xs group/img cursor-pointer py-0.5" title="Click to view full image">
              <div class="relative w-8 h-8 rounded-xl overflow-hidden border border-slate-300 bg-slate-100 shrink-0 shadow-2xs group-hover/img:border-emerald-500 transition">
                <img src="${url}" class="w-full h-full object-cover group-hover/img:scale-110 transition duration-150" onerror="this.parentElement.innerHTML='<div class=\\\'w-full h-full flex items-center justify-center text-slate-400 text-[10px]\\\'><i class=\\\'fa-regular fa-image\\\'></i></div>'">
              </div>
              <span class="font-mono text-[11px] truncate max-w-[130px] font-semibold text-slate-800 group-hover/img:text-emerald-700 group-hover/img:underline">${val}</span>
            </div>
          `;
        }
        return `
          <a href="${url}?download=1" target="_blank" onclick="event.stopPropagation()" class="px-2.5 py-1 rounded-xl bg-slate-100 hover:bg-emerald-50 text-slate-700 hover:text-emerald-800 border border-slate-200 hover:border-emerald-300 text-[11px] font-mono inline-flex items-center space-x-1.5 transition">
            <i class="fa-solid fa-paperclip text-[10px] text-slate-400"></i>
            <span class="truncate max-w-[130px] font-semibold">${val}</span>
          </a>
        `;
      }
      if (type === 'bool' || typeof val === 'boolean') {
        const isTrue = val === true || val === 1 || val === 'true';
        return isTrue
          ? '<span class="px-2 py-0.5 rounded-full bg-emerald-100 text-emerald-800 text-[10px] font-bold">TRUE</span>'
          : '<span class="px-2 py-0.5 rounded-full bg-slate-100 text-slate-500 text-[10px] font-bold">FALSE</span>';
      }
      if (typeof val === 'object') return `<span class="text-teal-700">${JSON.stringify(val)}</span>`;
      return String(val).replace(/</g, '&lt;').replace(/>/g, '&gt;');
    }

    function formatDateCell(iso) {
      if (!iso) return '';
      if (typeof iso === 'number' || (/^\d+$/.test(String(iso)) && String(iso).length >= 10)) {
        const num = Number(iso);
        const ms = num < 10000000000 ? num * 1000 : num;
        const d = new Date(ms);
        if (!isNaN(d.getTime())) {
          return d.toISOString().slice(0, 19).replace('T', ' ');
        }
      }
      return String(iso).slice(0, 19).replace('T', ' ');
    }

    function toggleSelectAll(checked) {
      if (checked) {
        currentRecords.forEach(r => selectedRecordIds.add(r.id));
      } else {
        selectedRecordIds.clear();
      }
      updateBulkDeleteBar();
      loadRecords();
    }

    function toggleSelectRecord(id, checked) {
      if (checked) selectedRecordIds.add(id);
      else selectedRecordIds.delete(id);
      updateBulkDeleteBar();
    }

    function updateBulkDeleteBar() {
      const bar = document.getElementById('bulkDeleteBar');
      if (selectedRecordIds.size > 0) {
        bar.classList.remove('hidden');
        document.getElementById('selectedCountText').textContent = `${selectedRecordIds.size} selected`;
      } else {
        bar.classList.add('hidden');
      }
    }

    async function deleteSelectedRecords() {
      if (!confirm(`Delete ${selectedRecordIds.size} selected record(s)?`)) return;
      for (const id of selectedRecordIds) {
        await fetch(`/api/collections/${currentCollection}/records/${id}`, { method: 'DELETE' });
      }
      selectedRecordIds.clear();
      updateBulkDeleteBar();
      showToast('Selected records deleted successfully');
      loadRecords();
    }

    function copyToClipboard(text) {
      navigator.clipboard.writeText(text);
      showToast(`Copied to clipboard: "${text}"`);
    }

    // ================= RECORD STUDIO PANE (CREATE / EDIT - NO DIALOG) =================
    async function openRecordDrawer(recordId) {
      editingRecordId = recordId;
      isRecordJsonMode = false;
      const formEl = document.getElementById('recordFormContainer');
      const jsonEl = document.getElementById('recordJsonContainer');
      const alertEl = document.getElementById('recordDrawerAlert');
      if (formEl) formEl.classList.remove('hidden');
      if (jsonEl) jsonEl.classList.add('hidden');
      if (alertEl) alertEl.classList.add('hidden');
      const toggleBtnText = document.getElementById('toggleJsonBtnText');
      if (toggleBtnText) toggleBtnText.textContent = '{ } JSON Mode';

      const col = collections.find(c => c.name === currentCollection) || { name: currentCollection, schema: [] };
      const colBadge = document.getElementById('recordEditorColBadge');
      if (colBadge) colBadge.textContent = col.name;

      const titleEl = document.getElementById('recordEditorTitle');
      if (titleEl) {
        titleEl.textContent = recordId ? `Edit Record #${recordId}` : `Create New Record in "${col.name}"`;
      }
      const tabText = document.getElementById('wsTab_record_text');
      if (tabText) {
        tabText.textContent = recordId ? 'Edit Record' : 'New Record';
      }
      const saveBtnText = document.getElementById('saveRecordBtnText');
      if (saveBtnText) {
        saveBtnText.textContent = recordId ? 'Save Changes' : 'Create Record';
      }
      const delBtn = document.getElementById('drawerDeleteRecordBtn');
      if (delBtn) delBtn.classList.toggle('hidden', !recordId);

      let record = {};
      if (recordId) {
        try {
          const res = await fetch(`/api/collections/${currentCollection}/records/${recordId}`);
          if (res.ok) {
            const data = await res.json();
            record = data.item || data;
          }
        } catch (_) {}
      }

      renderRecordDrawerForm(col, record);

      const drawer = document.getElementById('recordDrawerModal');
      if (drawer) drawer.classList.remove('hidden');
    }

    function closeRecordEditorPage() {
      const drawer = document.getElementById('recordDrawerModal');
      if (drawer) drawer.classList.add('hidden');
    }

    function renderRecordDrawerForm(col, record) {
      const container = document.getElementById('recordFormContainer');
      let html = `
        <div>
          <label class="block text-xs font-bold text-slate-600 mb-1 flex items-center">
            <i class="fa-solid fa-fingerprint text-[10px] mr-1.5 text-emerald-600"></i> Record ID (Primary Key)
          </label>
          <input type="text" id="recField_id" value="${record.id || ''}" ${editingRecordId ? 'readonly class="w-full bg-slate-100 border border-slate-200 rounded-xl px-3.5 py-2 text-xs font-mono text-slate-500 cursor-not-allowed"' : 'class="w-full bg-slate-50 border border-slate-300 rounded-xl px-3.5 py-2 text-xs font-mono text-slate-800 focus:bg-white focus:outline-none focus:border-emerald-600" placeholder="Leave blank for auto-generation"'}>
        </div>
      `;

      if (col.type === 'auth') {
        html += `
          <div class="bg-emerald-50/50 border border-emerald-200/80 rounded-2xl p-4 space-y-3.5">
            <div class="text-[11px] font-extrabold uppercase tracking-wider text-emerald-800 flex items-center">
              <i class="fa-solid fa-user-shield mr-1.5"></i> Auth Account Credentials (Padi Framework)
            </div>
            <div class="grid grid-cols-3 gap-3">
              <div>
                <label class="block text-xs font-bold text-slate-700 mb-1">name</label>
                <input type="text" id="recField_name" value="${record.name || ''}" class="w-full bg-white border border-slate-300 rounded-xl px-3 py-2 text-xs font-mono focus:outline-none focus:border-emerald-600" placeholder="Full name">
              </div>
              <div>
                <label class="block text-xs font-bold text-slate-700 mb-1 flex items-center justify-between">
                  <span class="flex items-center"><i class="fa-solid fa-shield-halved text-[10px] mr-1 text-emerald-600"></i> role <span class="text-rose-500">*</span></span>
                  <a href="javascript:void(0)" onclick="openSettingsRoles()" class="text-[10px] text-emerald-600 hover:underline font-normal" title="Kelola daftar role di Settings">Edit Roles &rarr;</a>
                </label>
                <select id="recField_role" class="w-full bg-white border border-slate-300 rounded-xl px-3 py-2 text-xs font-mono font-bold text-slate-800 focus:outline-none focus:border-emerald-600 shadow-2xs">
                  ${AVAILABLE_ROLES.map(r => `
                    <option value="${r}" ${(record.role || 'user').toLowerCase() === r ? 'selected' : ''}>${r}</option>
                  `).join('')}
                </select>
              </div>
              <div>
                <label class="block text-xs font-bold text-slate-700 mb-1">status</label>
                <input type="text" id="recField_status" value="${record.status || 'active'}" class="w-full bg-white border border-slate-300 rounded-xl px-3 py-2 text-xs font-mono focus:outline-none focus:border-emerald-600" placeholder="active / suspended">
              </div>
            </div>
            <div class="grid grid-cols-2 gap-3">
              <div>
                <label class="block text-xs font-bold text-slate-700 mb-1">username</label>
                <input type="text" id="recField_username" value="${record.username || ''}" class="w-full bg-white border border-slate-300 rounded-xl px-3 py-2 text-xs font-mono focus:outline-none focus:border-emerald-600" placeholder="Auto-generated if empty">
              </div>
              <div>
                <label class="block text-xs font-bold text-slate-700 mb-1">email <span class="text-rose-500">*</span></label>
                <input type="email" id="recField_email" value="${record.email || ''}" class="w-full bg-white border border-slate-300 rounded-xl px-3 py-2 text-xs font-mono focus:outline-none focus:border-emerald-600" placeholder="user@example.com">
              </div>
            </div>
            <div class="grid grid-cols-2 gap-3">
              <div>
                <label class="block text-xs font-bold text-slate-700 mb-1">${editingRecordId ? 'New password (optional)' : 'Password *'}</label>
                <input type="password" id="recField_password" class="w-full bg-white border border-slate-300 rounded-xl px-3 py-2 text-xs font-mono focus:outline-none focus:border-emerald-600" placeholder="••••••••">
              </div>
              <div class="flex items-end pb-2">
                <label class="inline-flex items-center space-x-2 cursor-pointer text-xs font-bold text-slate-700">
                  <input type="checkbox" id="recField_verified" ${record.verified ? 'checked' : ''} class="rounded border-slate-300">
                  <span>Verified Account</span>
                </label>
              </div>
            </div>
          </div>
        `;
      }

      for (const f of (col.schema || [])) {
        const val = record[f.name];
        const reqStar = f.required ? '<span class="text-rose-500">*</span>' : '';
        const icon = FIELD_ICONS[f.type] || 'fa-solid fa-font';
        let inputEl = '';

        switch (f.type) {
          case 'number':
            inputEl = `<input type="number" step="any" id="recField_${f.name}" value="${val !== undefined && val !== null ? val : ''}" class="w-full bg-slate-50 border border-slate-300 rounded-xl px-3.5 py-2 text-xs font-mono focus:bg-white focus:outline-none focus:border-emerald-600" placeholder="0">`;
            break;
          case 'bool':
            inputEl = `
              <label class="inline-flex items-center space-x-2 cursor-pointer">
                <input type="checkbox" id="recField_${f.name}" ${val === true || val === 1 || val === 'true' ? 'checked' : ''} class="rounded border-slate-300 w-4 h-4">
                <span class="text-xs text-slate-700 font-semibold">True / Active</span>
              </label>
            `;
            break;
          case 'date':
            inputEl = `<input type="datetime-local" id="recField_${f.name}" value="${val ? String(val).slice(0, 16) : ''}" class="w-full bg-slate-50 border border-slate-300 rounded-xl px-3.5 py-2 text-xs font-mono focus:bg-white focus:outline-none focus:border-emerald-600">`;
            break;
          case 'json':
            inputEl = `<textarea id="recField_${f.name}" rows="4" class="w-full bg-slate-950 text-emerald-400 border border-slate-300 rounded-xl p-3 text-xs font-mono focus:outline-none" placeholder="{ }">${typeof val === 'object' && val !== null ? JSON.stringify(val, null, 2) : (val || '')}</textarea>`;
            break;
          case 'file':
            const currentFile = val || '';
            const isImg = /\.(jpg|jpeg|png|gif|webp|svg)$/i.test(currentFile);
            const fileUrl = `/api/files/${currentCollection}/${editingRecordId}/${currentFile}`;
            inputEl = `
              <div class="space-y-2.5">
                ${editingRecordId && currentFile ? `
                  <div class="flex items-center justify-between p-3 rounded-2xl bg-slate-50 border border-slate-200 shadow-2xs">
                    <div class="flex items-center space-x-3 min-w-0">
                      ${isImg ? `
                        <div onclick="openImageLightbox('${fileUrl}', '${currentFile}')" class="w-12 h-12 rounded-xl overflow-hidden border border-slate-300 bg-white shadow-2xs shrink-0 cursor-pointer group/card relative" title="Click to view preview">
                          <img src="${fileUrl}" class="w-full h-full object-cover group-hover/card:scale-110 transition duration-150">
                          <div class="absolute inset-0 bg-slate-900/30 opacity-0 group-hover/card:opacity-100 flex items-center justify-center text-white text-xs transition"><i class="fa-solid fa-magnifying-glass-plus"></i></div>
                        </div>
                      ` : `
                        <div class="w-12 h-12 rounded-xl bg-indigo-50 border border-indigo-200 flex items-center justify-center text-indigo-600 shrink-0 text-lg">
                          <i class="fa-solid fa-file"></i>
                        </div>
                      `}
                      <div class="min-w-0 truncate">
                        <span class="text-xs font-mono font-bold text-slate-800 truncate block">${currentFile}</span>
                        <div class="flex items-center space-x-2 mt-0.5">
                          ${isImg ? `<button type="button" onclick="openImageLightbox('${fileUrl}', '${currentFile}')" class="text-[11px] font-semibold text-emerald-700 hover:underline flex items-center space-x-1"><i class="fa-solid fa-eye text-[10px]"></i><span>View High-Res</span></button>` : ''}
                          <a href="${fileUrl}?download=1" target="_blank" class="text-[11px] font-semibold text-indigo-700 hover:underline flex items-center space-x-1"><i class="fa-solid fa-download text-[10px]"></i><span>Download</span></a>
                        </div>
                      </div>
                    </div>
                    <button type="button" onclick="deleteCurrentRecordFile('${f.name}', '${currentFile}')" class="px-2.5 py-1 text-xs font-bold text-rose-600 hover:bg-rose-50 border border-transparent hover:border-rose-200 rounded-xl transition shrink-0" title="Delete attachment">
                      <i class="fa-solid fa-trash-can mr-1 text-[11px]"></i> Delete
                    </button>
                  </div>
                ` : ''}
                <!-- Live Instant Preview of newly chosen file -->
                <div id="newFilePreview_${f.name}" class="hidden p-2 rounded-xl bg-emerald-50/80 border border-emerald-200 flex items-center space-x-2.5">
                  <img id="newFilePreviewImg_${f.name}" src="" class="w-10 h-10 rounded-lg object-cover border border-emerald-300 bg-white shrink-0">
                  <div class="min-w-0 truncate text-xs">
                    <span id="newFilePreviewName_${f.name}" class="font-mono font-bold text-emerald-950 truncate block"></span>
                    <span id="newFilePreviewSize_${f.name}" class="text-[10px] text-emerald-700"></span>
                  </div>
                </div>
                <div class="flex items-center space-x-2">
                  <input type="file" id="recFieldFile_${f.name}" data-field-name="${f.name}" onchange="previewChosenFile(this, '${f.name}')" class="flex-1 text-xs text-slate-700 file:mr-3 file:py-1.5 file:px-3 file:rounded-xl file:border-0 file:text-xs file:font-bold file:bg-emerald-100 file:text-emerald-800 hover:file:bg-emerald-200 cursor-pointer border border-slate-300 rounded-xl bg-white p-1.5 focus:outline-none">
                  <input type="hidden" id="recField_${f.name}" value="${currentFile}">
                </div>
              </div>
            `;
            break;
          default:
            if (f.name.toLowerCase() === 'role') {
              const currentRoleVal = (val !== undefined && val !== null ? String(val) : 'user').toLowerCase();
              inputEl = `
                <select id="recField_${f.name}" class="w-full bg-white border border-slate-300 rounded-xl px-3.5 py-2 text-xs font-mono font-bold text-slate-800 focus:outline-none focus:border-emerald-600 shadow-2xs">
                  ${AVAILABLE_ROLES.map(r => `
                    <option value="${r}" ${currentRoleVal === r ? 'selected' : ''}>${r}</option>
                  `).join('')}
                </select>
              `;
            } else {
              inputEl = `<input type="${f.type === 'email' ? 'email' : (f.type === 'url' ? 'url' : 'text')}" id="recField_${f.name}" value="${val !== undefined && val !== null ? val : ''}" class="w-full bg-slate-50 border border-slate-300 rounded-xl px-3.5 py-2 text-xs focus:bg-white focus:outline-none focus:border-emerald-600" placeholder="Enter ${f.name}...">`;
            }
            break;
        }

        html += `
          <div>
            <label class="block text-xs font-bold text-slate-700 mb-1 flex items-center justify-between">
              <span><i class="${icon} text-[10px] text-emerald-600 mr-1.5"></i> ${f.name} ${reqStar}</span>
              <span class="text-[10px] font-mono text-slate-400 uppercase">${f.type}</span>
            </label>
            ${inputEl}
          </div>
        `;
      }

      container.innerHTML = html;
    }

    function collectRecordFormPayload() {
      const col = collections.find(c => c.name === currentCollection) || { schema: [] };
      const payload = {};

      const customId = document.getElementById('recField_id')?.value.trim();
      if (!editingRecordId && customId) payload.id = customId;

      if (col.type === 'auth') {
        const n = document.getElementById('recField_name')?.value.trim();
        const u = document.getElementById('recField_username')?.value.trim();
        const e = document.getElementById('recField_email')?.value.trim();
        const p = document.getElementById('recField_password')?.value;
        const r = document.getElementById('recField_role')?.value.trim();
        const s = document.getElementById('recField_status')?.value.trim();
        const v = document.getElementById('recField_verified')?.checked;
        if (n) payload.name = n;
        if (u) payload.username = u;
        if (e) payload.email = e;
        if (p) payload.password = p;
        if (r) payload.role = r;
        if (s) payload.status = s;
        payload.verified = v ? 1 : 0;
      }

      for (const f of (col.schema || [])) {
        const el = document.getElementById(`recField_${f.name}`);
        if (!el) continue;
        if (f.type === 'bool') {
          payload[f.name] = el.checked;
        } else if (f.type === 'number') {
          if (el.value !== '') payload[f.name] = Number(el.value);
        } else if (f.type === 'json') {
          try { payload[f.name] = JSON.parse(el.value || '{}'); }
          catch (_) { payload[f.name] = el.value; }
        } else {
          payload[f.name] = el.value;
        }
      }
      return payload;
    }

    function toggleRecordJsonMode() {
      isRecordJsonMode = !isRecordJsonMode;
      const formEl = document.getElementById('recordFormContainer');
      const jsonEl = document.getElementById('recordJsonContainer');
      const btn = document.getElementById('toggleJsonBtn');

      if (isRecordJsonMode) {
        document.getElementById('recordPayloadJson').value = JSON.stringify(collectRecordFormPayload(), null, 2);
        formEl.classList.add('hidden');
        jsonEl.classList.remove('hidden');
        btn.textContent = 'Visual Form';
      } else {
        formEl.classList.remove('hidden');
        jsonEl.classList.add('hidden');
        btn.textContent = '{ } JSON Mode';
      }
    }

    async function saveRecordDrawer() {
      const alertEl = document.getElementById('recordDrawerAlert');
      alertEl.classList.add('hidden');

      let payload = {};
      if (isRecordJsonMode) {
        try {
          payload = JSON.parse(document.getElementById('recordPayloadJson').value || '{}');
        } catch (e) {
          alertEl.textContent = 'Invalid JSON: ' + e.message;
          alertEl.classList.remove('hidden');
          return;
        }
      } else {
        payload = collectRecordFormPayload();
      }

      const url = editingRecordId
        ? `/api/collections/${currentCollection}/records/${editingRecordId}`
        : `/api/collections/${currentCollection}/records`;
      const method = editingRecordId ? 'PATCH' : 'POST';

      const fileInputs = document.querySelectorAll('#recordFormContainer input[type="file"]');
      let hasFiles = false;
      fileInputs.forEach(fi => { if (fi.files && fi.files.length > 0) hasFiles = true; });

      let bodyData;
      let headers = {
        'Authorization': 'Bearer ' + (localStorage.getItem('padi_admin_token') || '')
      };

      if (!isRecordJsonMode && hasFiles) {
        const formData = new FormData();
        for (const [k, v] of Object.entries(payload)) {
          if (v !== undefined && v !== null) {
            formData.append(k, typeof v === 'object' ? JSON.stringify(v) : v);
          }
        }
        fileInputs.forEach(fi => {
          const fieldName = fi.getAttribute('data-field-name');
          if (fieldName && fi.files && fi.files[0]) {
            formData.append(fieldName, fi.files[0]);
          }
        });
        bodyData = formData;
      } else {
        headers['Content-Type'] = 'application/json';
        bodyData = JSON.stringify(payload);
      }

      try {
        const res = await fetch(url, {
          method,
          headers,
          body: bodyData
        });
        const data = await res.json();
        if (res.ok) {
          closeRecordEditorPage();
          showToast(editingRecordId ? 'Record updated successfully' : 'New record created successfully');
          loadRecords();
        } else {
          alertEl.textContent = data.message || 'Failed to save record';
          alertEl.classList.remove('hidden');
        }
      } catch (err) {
        alertEl.textContent = err.message;
        alertEl.classList.remove('hidden');
      }
    }

    async function deleteCurrentRecordFile(fieldName, filename) {
      if (!editingRecordId) return;
      if (!confirm(`Delete file "${filename}"?`)) return;
      try {
        const res = await fetch(`/api/files/${currentCollection}/${editingRecordId}/${filename}`, {
          method: 'DELETE',
          headers: {
            'Authorization': 'Bearer ' + (localStorage.getItem('padi_admin_token') || '')
          }
        });
        if (res.ok) {
          showToast(`File "${filename}" deleted.`);
          openRecordDrawer(editingRecordId);
          loadRecords();
        } else {
          showToast('Failed to delete file', 'error');
        }
      } catch (err) {
        showToast(err.message, 'error');
      }
    }

    let currentLightboxUrl = '';

    function openImageLightbox(url, filename) {
      currentLightboxUrl = url;
      const modal = document.getElementById('imageLightboxModal');
      const img = document.getElementById('lightboxImage');
      const nameEl = document.getElementById('lightboxFilename');
      const dimEl = document.getElementById('lightboxDimensions');
      const dlBtn = document.getElementById('lightboxDownloadBtn');
      const tabBtn = document.getElementById('lightboxOpenTabBtn');

      if (!modal || !img) return;

      nameEl.textContent = filename || 'Image Preview';
      dimEl.innerHTML = '<i class="fa-solid fa-circle-notch fa-spin mr-1"></i>Loading dimensions...';
      img.src = url;
      img.onload = () => {
        dimEl.textContent = `${img.naturalWidth} × ${img.naturalHeight} px`;
      };
      img.onerror = () => {
        dimEl.textContent = 'Could not load image';
      };

      if (dlBtn) dlBtn.href = `${url}?download=1`;
      if (tabBtn) tabBtn.href = url;

      modal.classList.remove('hidden');
    }

    function copyLightboxUrl() {
      if (!currentLightboxUrl) return;
      const fullUrl = window.location.origin + currentLightboxUrl;
      navigator.clipboard.writeText(fullUrl).then(() => {
        showToast('Direct file URL copied to clipboard!');
      }).catch(() => {
        showToast(fullUrl);
      });
    }

    function previewChosenFile(input, fieldName) {
      const container = document.getElementById(`newFilePreview_${fieldName}`);
      const img = document.getElementById(`newFilePreviewImg_${fieldName}`);
      const nameEl = document.getElementById(`newFilePreviewName_${fieldName}`);
      const sizeEl = document.getElementById(`newFilePreviewSize_${fieldName}`);
      if (!input.files || input.files.length === 0) {
        if (container) container.classList.add('hidden');
        return;
      }
      const file = input.files[0];
      const isImg = file.type.startsWith('image/');
      if (nameEl) nameEl.textContent = file.name;
      if (sizeEl) sizeEl.textContent = `${(file.size / 1024).toFixed(1)} KB — Ready to upload`;
      if (isImg && img) {
        img.src = URL.createObjectURL(file);
        img.classList.remove('hidden');
      } else if (img) {
        img.classList.add('hidden');
      }
      if (container) container.classList.remove('hidden');
    }

    async function deleteRecordFromDrawer() {
      if (!editingRecordId) return;
      if (!confirm(`Delete record ${editingRecordId}?`)) return;
      const res = await fetch(`/api/collections/${currentCollection}/records/${editingRecordId}`, { method: 'DELETE' });
      if (res.ok) {
        closeRecordEditorPage();
        showToast('Record deleted successfully');
        loadRecords();
      }
    }

    // ================= ROLE BASED ACCESS CONTROL (RBAC) & ACCESS RULES =================
    let AVAILABLE_ROLES = ['admin', 'manager', 'editor', 'moderator', 'staff', 'creator', 'user'];

    function getRolesFromRuleText(ruleText) {
      const roles = [];
      const regex = /(?:@request\.auth\.role|users\.role|role)\s*=\s*['"]([^'"]+)['"]/g;
      let match;
      while ((match = regex.exec(ruleText || '')) !== null) {
        if (!roles.includes(match[1])) roles.push(match[1]);
      }
      return roles;
    }

    function toggleRoleInRule(inputId, role) {
      const el = document.getElementById(inputId);
      if (!el) return;
      let text = (el.value || '').trim();
      const roleSnippet = `@request.auth.role = '${role}'`;
      if (!text || text === 'public' || text === '@request.auth.id != ""' || text === "@request.auth.id != ''") {
        el.value = roleSnippet;
        updateRoleDropdownUI(inputId);
        return;
      }
      let segments = text.split('||').map(s => s.trim()).filter(Boolean);
      const existingIdx = segments.findIndex(s => getRolesFromRuleText(s).includes(role));
      if (existingIdx !== -1) {
        segments.splice(existingIdx, 1);
        el.value = segments.join(' || ');
      } else {
        segments.push(roleSnippet);
        el.value = segments.join(' || ');
      }
      updateRoleDropdownUI(inputId);
    }

    function addCustomRoleToRule(inputId) {
      const inp = document.getElementById(`customRoleInput_${inputId}`);
      if (!inp) return;
      const role = inp.value.trim().toLowerCase();
      if (!role) return;
      inp.value = '';
      if (!AVAILABLE_ROLES.includes(role)) AVAILABLE_ROLES.push(role);
      toggleRoleInRule(inputId, role);
    }

    function updateRoleDropdownUI(inputId) {
      const el = document.getElementById(inputId);
      const listEl = document.getElementById(`roleOptionsList_${inputId}`);
      const btnLabel = document.getElementById(`roleBtnLabel_${inputId}`);
      if (!el) return;
      const currentRoles = getRolesFromRuleText(el.value);
      if (btnLabel) {
        if (currentRoles.length === 0) btnLabel.textContent = 'Roles ▾';
        else if (currentRoles.length === 1) btnLabel.textContent = `Role: ${currentRoles[0]} ▾`;
        else btnLabel.textContent = `Roles (${currentRoles.length}) ▾`;
      }
      if (listEl) {
        const allRoles = Array.from(new Set([...AVAILABLE_ROLES, ...currentRoles]));
        listEl.innerHTML = allRoles.map(role => {
          const isSelected = currentRoles.includes(role);
          return `
            <label class="flex items-center justify-between px-2 py-1.5 rounded-lg hover:bg-slate-50 cursor-pointer text-xs transition select-none">
              <span class="flex items-center space-x-1.5 font-mono ${isSelected ? 'text-amber-800 font-bold' : 'text-slate-700'}">
                <span class="w-1.5 h-1.5 rounded-full ${isSelected ? 'bg-amber-500' : 'bg-slate-300'}"></span>
                <span>${role}</span>
              </span>
              <input type="checkbox" ${isSelected ? 'checked' : ''} onchange="toggleRoleInRule('${inputId}', '${role}')" class="rounded border-slate-300 text-amber-600 focus:ring-amber-500 w-3.5 h-3.5">
            </label>
          `;
        }).join('');
      }
    }

    function toggleRoleDropdown(inputId) {
      const menu = document.getElementById(`roleDropdownMenu_${inputId}`);
      if (!menu) return;
      const isHidden = menu.classList.contains('hidden');
      document.querySelectorAll('[id^="roleDropdownMenu_"]').forEach(m => m.classList.add('hidden'));
      if (isHidden) {
        updateRoleDropdownUI(inputId);
        menu.classList.remove('hidden');
      } else {
        menu.classList.add('hidden');
      }
    }

    document.addEventListener('click', function(e) {
      if (!e.target.closest('[id^="roleDropdownWrapper_"]')) {
        document.querySelectorAll('[id^="roleDropdownMenu_"]').forEach(m => m.classList.add('hidden'));
      }
    });

    // ================= SCHEMA ARCHITECT & TABLE BUILDER (NO DIALOG) =================
    function applyRulePreset(inputId, snippet, mode = 'append') {
      const el = document.getElementById(inputId);
      if (!el) return;
      if (!snippet) {
        el.value = '';
        updateRoleDropdownUI(inputId);
        return;
      }
      const current = el.value.trim();

      if (mode === 'set' || !current || current === 'public' || current === '@request.auth.id != ""' || current === "@request.auth.id != ''") {
        el.value = snippet;
        updateRoleDropdownUI(inputId);
        return;
      }

      // Check if snippet is already present in || list
      const segments = current.split('||').map(s => s.trim()).filter(Boolean);
      if (segments.includes(snippet)) {
        if (segments.length > 1) {
          el.value = segments.filter(s => s !== snippet).join(' || ');
        } else {
          el.value = '';
        }
        updateRoleDropdownUI(inputId);
        return;
      }

      el.value = `${current} || ${snippet}`;
      updateRoleDropdownUI(inputId);
    }

    function openCollectionDrawer(isEdit, tab = 'fields') {
      if (isEdit) {
        openSchemaEditorPage(tab);
      } else {
        openCreateTablePage();
      }
    }

    let newTableFields = [];

    function openCreateTablePage() {
      const tabBtn = document.getElementById('wsTab_create_table');
      if (tabBtn) tabBtn.classList.remove('hidden');

      const alertEl = document.getElementById('newColAlert');
      if (alertEl) alertEl.classList.add('hidden');
      const nameInput = document.getElementById('newColNameInput');
      if (nameInput) nameInput.value = '';
      const typeInput = document.getElementById('newColTypeInput');
      if (typeInput) typeInput.value = 'base';
      const rtInput = document.getElementById('newColRealtimeInput');
      if (rtInput) rtInput.checked = true;
      const relInput = document.getElementById('newColAutoRelationsInput');
      if (relInput) relInput.checked = false;
      const blameInput = document.getElementById('newColBlameableInput');
      if (blameInput) blameInput.checked = true;

      const ruleList = document.getElementById('newRuleList');
      if (ruleList) ruleList.value = 'public';
      const ruleView = document.getElementById('newRuleView');
      if (ruleView) ruleView.value = 'public';
      const ruleCreate = document.getElementById('newRuleCreate');
      if (ruleCreate) ruleCreate.value = '@request.auth.id != ""';
      const ruleUpdate = document.getElementById('newRuleUpdate');
      if (ruleUpdate) ruleUpdate.value = '@request.auth.id != ""';
      const ruleDelete = document.getElementById('newRuleDelete');
      if (ruleDelete) ruleDelete.value = '@request.auth.id != ""';

      ['newRuleList', 'newRuleView', 'newRuleCreate', 'newRuleUpdate', 'newRuleDelete'].forEach(id => updateRoleDropdownUI(id));

      currentSchemaDbEngine = activeDbDriver || 'sqlite';
      newTableFields = [];
      renderNewTableSystemFields();
      renderSchemaDataTypeButtons();
      renderNewTableFields();

      const modal = document.getElementById('newCollectionModal');
      if (modal) modal.classList.remove('hidden');
    }

    function cancelCreateTablePage() {
      const modal = document.getElementById('newCollectionModal');
      if (modal) modal.classList.add('hidden');
    }

    function renderNewColAuthInfo() {
      renderNewTableSystemFields();
    }

    function renderNewTableSystemFields() {
      const container = document.getElementById('newColSystemFieldsContainer');
      if (!container) return;
      const type = document.getElementById('newColTypeInput')?.value || 'base';
      const isBlameable = document.getElementById('newColBlameableInput')?.checked !== false;

      let fields = [];
      if (type === 'auth') {
        fields = [
          { name: 'id', type: 'text', sql: 'VARCHAR(255) PK', desc: 'Primary Key unik record user (Auto-generated)', icon: 'fa-solid fa-fingerprint', isBlameable: false },
          { name: 'email', type: 'email', sql: 'VARCHAR(255) UNIQUE', desc: 'Email kredensial login (Wajib unik)', icon: 'fa-solid fa-envelope', isBlameable: false },
          { name: 'password', type: 'password', sql: 'VARCHAR(255) / TEXT', desc: 'Hash password terenkripsi bcrypt', icon: 'fa-solid fa-key', isBlameable: false },
          { name: 'name', type: 'text', sql: 'VARCHAR(255)', desc: 'Nama lengkap tampilan akun user', icon: 'fa-solid fa-id-card', isBlameable: false },
          { name: 'username', type: 'text', sql: 'VARCHAR(50)', desc: 'Handle unik / username login pengguna', icon: 'fa-solid fa-at', isBlameable: false },
          { name: 'role', type: 'text', sql: 'VARCHAR(50)', desc: 'Role otorisasi akses (default: "user")', icon: 'fa-solid fa-shield-halved', isBlameable: false },
          { name: 'status', type: 'text', sql: 'VARCHAR(20)', desc: 'Status akun pengguna (default: "active")', icon: 'fa-solid fa-circle-check', isBlameable: false },
          { name: 'verified', type: 'bool', sql: 'INTEGER', desc: 'Status verifikasi email user (0 / 1)', icon: 'fa-solid fa-badge-check', isBlameable: false },
          { name: 'tokenKey', type: 'text', sql: 'VARCHAR(255)', desc: 'Kunci enkripsi token sesi & auth', icon: 'fa-solid fa-lock', isBlameable: false },
          { name: 'created_at', type: 'number', sql: 'BIGINT / TIMESTAMP', desc: 'Timestamp saat akun pertama dibuat', icon: 'fa-solid fa-calendar-plus', isBlameable: false },
          { name: 'updated_at', type: 'number', sql: 'BIGINT / TIMESTAMP', desc: 'Timestamp saat profil akun diubah', icon: 'fa-solid fa-clock-rotate-left', isBlameable: false },
          { 
            name: 'created_by', 
            type: 'text', 
            sql: 'VARCHAR(255)', 
            desc: isBlameable ? 'ID admin/user pembuat akun (Blameable aktif)' : 'ID admin/user pembuat akun (Blameable nonaktif)', 
            icon: 'fa-solid fa-user-check', 
            isBlameable: true 
          },
          { 
            name: 'updated_by', 
            type: 'text', 
            sql: 'VARCHAR(255)', 
            desc: isBlameable ? 'ID admin/user pengubah akun (Blameable aktif)' : 'ID admin/user pengubah akun (Blameable nonaktif)', 
            icon: 'fa-solid fa-user-pen', 
            isBlameable: true 
          }
        ];
      } else {
        // Base Table
        fields = [
          { name: 'id', type: 'text', sql: 'VARCHAR(255) PK', desc: 'Primary Key unik record (Auto-generated)', icon: 'fa-solid fa-fingerprint', isBlameable: false },
          { name: 'created_at', type: 'number', sql: 'BIGINT / TIMESTAMP', desc: 'Unix timestamp otomatis saat record di-insert', icon: 'fa-solid fa-calendar-plus', isBlameable: false },
          { name: 'updated_at', type: 'number', sql: 'BIGINT / TIMESTAMP', desc: 'Unix timestamp otomatis saat record di-update', icon: 'fa-solid fa-clock-rotate-left', isBlameable: false },
          { 
            name: 'created_by', 
            type: 'text', 
            sql: 'VARCHAR(255)', 
            desc: isBlameable ? 'ID user pembuat record (Blameable aktif, dilewati jika unauthenticated)' : 'ID user pembuat record (Blameable nonaktif — centang opsi Blameable di atas)', 
            icon: 'fa-solid fa-user-check', 
            isBlameable: true 
          },
          { 
            name: 'updated_by', 
            type: 'text', 
            sql: 'VARCHAR(255)', 
            desc: isBlameable ? 'ID user pengubah record (Blameable aktif, dilewati jika unauthenticated)' : 'ID user pengubah record (Blameable nonaktif — centang opsi Blameable di atas)', 
            icon: 'fa-solid fa-user-pen', 
            isBlameable: true 
          }
        ];
      }

      const countBadge = document.getElementById('newColSystemCountBadge');
      if (countBadge) countBadge.textContent = `${fields.length} auto-fields`;

      container.innerHTML = fields.map(f => {
        let badgeHtml = '';
        if (f.isBlameable) {
          if (isBlameable) {
            badgeHtml = `<span class="inline-flex items-center text-[10px] text-emerald-700 font-semibold px-2 py-0.5 rounded-full bg-emerald-50 border border-emerald-200 shrink-0 ml-1.5"><i class="fa-solid fa-circle-check text-[9px] mr-1 text-emerald-600"></i>Blameable: ON</span>`;
          } else {
            badgeHtml = `<span class="inline-flex items-center text-[10px] text-slate-500 font-semibold px-2 py-0.5 rounded-full bg-slate-100 border border-slate-200 shrink-0 ml-1.5"><i class="fa-solid fa-toggle-off text-[9px] mr-1 text-slate-400"></i>Blameable: OFF</span>`;
          }
        } else {
          badgeHtml = `<span class="inline-flex items-center text-[10px] text-emerald-700 font-semibold px-1.5 py-0.5 rounded bg-emerald-50 border border-emerald-200 shrink-0 ml-1.5" title="Otomatis diisi dan dikelola oleh PadiBase"><i class="fa-solid fa-lock text-[8px] mr-1 text-emerald-600"></i>Auto</span>`;
        }

        return `
        <div class="flex items-center justify-between p-2 rounded-xl bg-white border border-slate-200 shadow-2xs hover:border-slate-300 transition min-w-0 ${f.isBlameable && !isBlameable ? 'opacity-75' : ''}">
          <div class="flex items-center space-x-2.5 min-w-0">
            <div class="w-7 h-7 rounded-lg ${f.isBlameable && !isBlameable ? 'bg-slate-100 text-slate-500 border-slate-200' : 'bg-emerald-50 text-emerald-700 border-emerald-100'} flex items-center justify-center text-xs shrink-0 border">
              <i class="${f.icon}"></i>
            </div>
            <div class="min-w-0">
              <div class="flex items-center space-x-1.5 flex-wrap">
                <span class="font-mono font-bold text-slate-800 text-xs">${f.name}</span>
                <span class="text-[9px] font-mono px-1.5 py-0.2 rounded bg-slate-100 text-slate-600 border border-slate-200">${f.sql}</span>
              </div>
              <p class="text-[10px] text-slate-400 truncate mt-0.5" title="${f.desc}">${f.desc}</p>
            </div>
          </div>
          ${badgeHtml}
        </div>
        `;
      }).join('');
    }

    function renderNewTableFields() {
      const container = document.getElementById('newColFieldsList');
      if (!container) return;

      renderSchemaDataTypeButtons();

      if (newTableFields.length === 0) {
        container.innerHTML = `<div class="py-6 text-center text-xs text-slate-400 border border-dashed border-slate-300 rounded-2xl">No custom columns added yet. Click one of the database data type buttons below to add columns.</div>`;
        return;
      }

      const engineCfg = DB_ENGINE_TYPES[currentSchemaDbEngine] || DB_ENGINE_TYPES['sqlite'];

      container.innerHTML = newTableFields.map((f, idx) => {
        const isRel = f.type === 'relation';
        const targetTable = (f.options && (f.options.targetTable || f.options.collectionId)) || '';
        const targetField = (f.options && f.options.targetField) || 'id';
        const targetCol = collections.find(c => c.name === targetTable);
        const targetFieldsList = ['id'];
        if (targetCol && Array.isArray(targetCol.schema)) {
          targetCol.schema.forEach(sf => {
            if (sf.name && !targetFieldsList.includes(sf.name)) targetFieldsList.push(sf.name);
          });
        }
        const sqlType = getSqlTypeForField(f, currentSchemaDbEngine);
        const isVarchar = isVarcharType(f, currentSchemaDbEngine);
        const varcharLen = getVarcharLength(f);

        return `
        <div class="space-y-2 bg-slate-50 p-2.5 rounded-2xl border ${isRel ? 'border-teal-300 bg-teal-50/20 shadow-2xs' : 'border-slate-200'} text-xs transition min-w-0">
          ${isRel ? `
          <div class="flex flex-wrap items-center justify-between gap-1.5 bg-teal-50 border border-teal-200/90 rounded-xl px-2.5 py-1.5 text-[11px] text-teal-950 min-w-0">
            <div class="flex items-center space-x-1.5 flex-wrap gap-y-1 min-w-0">
              <span class="font-bold flex items-center space-x-1 text-teal-900 shrink-0">
                <i class="fa-solid fa-diagram-project text-teal-600"></i>
                <span>References:</span>
              </span>
              <div class="flex items-center space-x-1 min-w-0">
                <select onchange="updateNewTableRelationTargetTable(${idx}, this.value)" class="bg-white border border-teal-300 rounded-lg px-2 py-1 font-mono text-[11px] font-bold text-teal-950 focus:outline-none focus:ring-1 focus:ring-teal-500 max-w-[150px] truncate">
                  <option value="">-- Target Table --</option>
                  ${collections.map(c => `<option value="${c.name}" ${targetTable === c.name ? 'selected' : ''}>${c.name}</option>`).join('')}
                </select>
                <span class="font-bold text-teal-700">.</span>
                <select onchange="updateNewTableRelationTargetField(${idx}, this.value)" class="bg-white border border-teal-300 rounded-lg px-2 py-1 font-mono text-[11px] font-bold text-teal-950 focus:outline-none focus:ring-1 focus:ring-teal-500 max-w-[90px] truncate">
                  ${targetFieldsList.map(fl => `<option value="${fl}" ${targetField === fl ? 'selected' : ''}>${fl}</option>`).join('')}
                </select>
              </div>
            </div>
            <span class="hidden md:inline-flex items-center text-[10px] text-teal-800 font-medium bg-white/90 border border-teal-200/70 rounded-md px-2 py-0.5 shadow-2xs shrink-0">
              <i class="fa-solid fa-shield-halved text-teal-600 mr-1"></i>Auto Foreign Key
            </span>
          </div>
          ` : ''}
          <div class="flex items-center space-x-2 min-w-0 w-full">
            <div class="relative shrink-0">
              <select onchange="onNewTableFieldTypeChange(${idx}, this.value)" class="appearance-none pl-6 pr-5 py-1.5 rounded-xl ${isRel ? 'bg-teal-100 hover:bg-teal-200 text-teal-900 border-teal-300' : 'bg-emerald-100 hover:bg-emerald-200 text-emerald-900 border-emerald-300'} font-mono font-bold text-[11px] border cursor-pointer focus:outline-none focus:ring-2 focus:ring-teal-500 transition max-w-[140px] truncate" title="Change column data type">
                ${engineCfg.types.map(t => {
                  const val = `${t.type}:${t.sqlType}`;
                  const isSelected = f.type === t.type && (!f.options?.sqlType || f.options.sqlType === t.sqlType);
                  const optLabel = (t.label.toUpperCase() === (t.sub || '').toUpperCase() || t.sub === 'FOREIGN KEY') ? t.label : `${t.label} (${t.sub})`;
                  return `<option value="${val}" ${isSelected ? 'selected' : ''}>${optLabel}</option>`;
                }).join('')}
              </select>
              <i class="${FIELD_ICONS[f.type] || 'fa-solid fa-font'} ${isRel ? 'text-teal-700' : 'text-emerald-700'} absolute left-2 top-1/2 -translate-y-1/2 pointer-events-none text-[11px]"></i>
              <i class="fa-solid fa-chevron-down ${isRel ? 'text-teal-700' : 'text-emerald-700'} absolute right-1.5 top-1/2 -translate-y-1/2 pointer-events-none text-[9px]"></i>
            </div>
            <input type="text" value="${f.name || ''}" oninput="newTableFields[${idx}].name = this.value.trim()" placeholder="column_name" class="flex-1 min-w-[70px] bg-white border border-slate-300 rounded-xl px-2.5 py-1.5 font-mono text-xs text-slate-900 focus:outline-none focus:border-teal-600">
            ${isVarchar ? `
            <div class="flex items-center space-x-1 shrink-0 bg-white border border-slate-300 rounded-xl px-1.5 py-1 shadow-2xs" title="Max character length (default: 255)">
              <span class="text-[10px] text-slate-400 font-semibold uppercase">Len:</span>
              <input type="number" min="1" max="65535" value="${varcharLen}" oninput="updateNewTableVarcharLength(${idx}, this.value)" class="w-10 text-center font-mono font-bold text-xs text-slate-800 focus:outline-none" placeholder="255">
            </div>
            ` : ''}
            <span id="newTableSqlTypeBadge_${idx}" class="px-2 py-0.5 rounded-md font-mono text-[10px] bg-white text-slate-700 font-bold border border-slate-200 shrink-0 shadow-2xs whitespace-nowrap" title="Physical SQL type in ${engineCfg.name}">
              SQL: ${sqlType}
            </span>
            <label class="inline-flex items-center space-x-1 text-[11px] font-semibold text-slate-600 cursor-pointer shrink-0" title="Required column">
              <input type="checkbox" ${f.required ? 'checked' : ''} onchange="newTableFields[${idx}].required = this.checked" class="rounded border-slate-300 text-teal-600">
              <span>Req</span>
            </label>
            <label class="inline-flex items-center space-x-1 text-[11px] font-semibold text-slate-600 cursor-pointer shrink-0" title="Unique column">
              <input type="checkbox" ${f.unique ? 'checked' : ''} onchange="newTableFields[${idx}].unique = this.checked" class="rounded border-slate-300 text-teal-600">
              <span>Uniq</span>
            </label>
            <button type="button" onclick="removeNewColField(${idx})" class="w-7 h-7 rounded-xl hover:bg-rose-50 text-slate-400 hover:text-rose-600 flex items-center justify-center transition shrink-0" title="Delete column">
              <i class="fa-solid fa-trash-can"></i>
            </button>
          </div>
        </div>
        `;
      }).join('');
    }

    function onNewTableFieldTypeChange(idx, val) {
      if (!newTableFields[idx]) return;
      if (val.includes(':')) {
        const [t, sql] = val.split(':');
        newTableFields[idx].type = t;
        newTableFields[idx].options = newTableFields[idx].options || {};
        if (sql.startsWith('VARCHAR')) {
          const len = getVarcharLength(newTableFields[idx]);
          newTableFields[idx].options.length = len;
          newTableFields[idx].options.sqlType = `VARCHAR(${len})`;
        } else {
          newTableFields[idx].options.sqlType = sql;
        }
      } else {
        newTableFields[idx].type = val;
      }
      if (newTableFields[idx].type === 'relation') {
        newTableFields[idx].options = newTableFields[idx].options || {};
        const defaultTarget = newTableFields[idx].options.targetTable || (collections[0]?.name || '');
        if (defaultTarget) {
          updateNewTableRelationTargetTable(idx, defaultTarget);
          return;
        }
      }
      renderNewTableFields();
    }

    function updateNewTableRelationTargetTable(idx, targetTable) {
      if (!newTableFields[idx]) return;
      newTableFields[idx].options = newTableFields[idx].options || {};
      newTableFields[idx].options.targetTable = targetTable;
      newTableFields[idx].options.collectionId = targetTable;
      newTableFields[idx].options.targetField = 'id';

      // Always auto-update relation column name to match selected target table
      if (targetTable) {
        newTableFields[idx].name = formatRelationFieldName(targetTable, newTableFields, idx);
      }
      renderNewTableFields();
    }

    function updateNewTableRelationTargetField(idx, targetField) {
      if (!newTableFields[idx]) return;
      newTableFields[idx].options = newTableFields[idx].options || {};
      newTableFields[idx].options.targetField = targetField;
      renderNewTableFields();
    }

    function addNewColField(type, sqlType) {
      const options = {};
      if (sqlType && sqlType.startsWith('VARCHAR')) {
        options.length = 255;
        options.sqlType = 'VARCHAR(255)';
      } else if (sqlType) {
        options.sqlType = sqlType;
      }
      let colName = `field_${newTableFields.length + 1}`;
      if (type === 'relation' && collections.length > 0) {
        const defaultTarget = collections[0].name;
        options.targetTable = defaultTarget;
        options.collectionId = defaultTarget;
        options.targetField = 'id';
        colName = formatRelationFieldName(defaultTarget, newTableFields);
      }
      newTableFields.push({
        name: colName,
        type,
        required: false,
        unique: false,
        options
      });
      renderNewTableFields();
    }

    function removeNewColField(idx) {
      newTableFields.splice(idx, 1);
      renderNewTableFields();
    }

    function applyNewTableSecurityProfile(profile) {
      const map = {
        recommended: ['public', 'public', '@request.auth.id != ""', '@request.auth.id != ""', '@request.auth.id != ""'],
        public_all: ['public', 'public', 'public', 'public', 'public'],
        private_all: ['@request.auth.id != ""', '@request.auth.id != ""', '@request.auth.id != ""', '@request.auth.id != ""', '@request.auth.id != ""']
      }[profile];
      if (!map) return;
      ['newRuleList', 'newRuleView', 'newRuleCreate', 'newRuleUpdate', 'newRuleDelete'].forEach((id, idx) => {
        const el = document.getElementById(id);
        if (el) {
          el.value = map[idx];
          updateRoleDropdownUI(id);
        }
      });
    }

    async function saveNewTablePage() {
      const alertEl = document.getElementById('newColAlert');
      if (alertEl) alertEl.classList.add('hidden');

      const name = document.getElementById('newColNameInput')?.value.trim().toLowerCase();
      const type = document.getElementById('newColTypeInput')?.value;
      if (!name) {
        if (alertEl) {
          alertEl.textContent = 'Table name is required';
          alertEl.classList.remove('hidden');
        }
        return;
      }

      const cleanSchema = newTableFields.filter(f => f.name && f.name.trim() !== '');
      const isBlameable = document.getElementById('newColBlameableInput')?.checked !== false;

      // Check if user accidentally duplicated a system auto-generated field
      const systemReservedNames = type === 'auth'
        ? ['id', 'email', 'password', 'passwordhash', 'name', 'username', 'role', 'status', 'verified', 'tokenkey', 'created_at', 'updated_at', 'created_by', 'updated_by']
        : ['id', 'created_at', 'updated_at', 'created_by', 'updated_by'];

      const dupField = cleanSchema.find(f => systemReservedNames.includes(f.name.toLowerCase().trim()));
      if (dupField) {
        if (alertEl) {
          alertEl.textContent = `Kolom "${dupField.name}" adalah auto-generated system field bawaan PadiBase. Anda tidak perlu menambahkannya di Custom Columns.`;
          alertEl.classList.remove('hidden');
        }
        return;
      }

      // Auto-populate created_at and updated_at into schema metadata
      if (type === 'base') {
        if (!cleanSchema.some(f => f.name.toLowerCase() === 'created_at')) {
          cleanSchema.push({
            name: 'created_at',
            type: 'number',
            required: false,
            unique: false,
            options: { sqlType: 'BIGINT' }
          });
        }
        if (!cleanSchema.some(f => f.name.toLowerCase() === 'updated_at')) {
          cleanSchema.push({
            name: 'updated_at',
            type: 'number',
            required: false,
            unique: false,
            options: { sqlType: 'BIGINT' }
          });
        }
        if (isBlameable) {
          if (!cleanSchema.some(f => f.name.toLowerCase() === 'created_by')) {
            cleanSchema.push({
              name: 'created_by',
              type: 'text',
              required: false,
              unique: false,
              options: { sqlType: 'VARCHAR(255)' }
            });
          }
          if (!cleanSchema.some(f => f.name.toLowerCase() === 'updated_by')) {
            cleanSchema.push({
              name: 'updated_by',
              type: 'text',
              required: false,
              unique: false,
              options: { sqlType: 'VARCHAR(255)' }
            });
          }
        }
      }

      const payload = {
        name,
        type,
        realtime: document.getElementById('newColRealtimeInput')?.checked,
        autoLoadRelations: document.getElementById('newColAutoRelationsInput')?.checked,
        blameable: isBlameable,
        schema: cleanSchema,
        listRule: document.getElementById('newRuleList')?.value.trim() || 'public',
        viewRule: document.getElementById('newRuleView')?.value.trim() || 'public',
        createRule: document.getElementById('newRuleCreate')?.value.trim() || '@request.auth.id != ""',
        updateRule: document.getElementById('newRuleUpdate')?.value.trim() || '@request.auth.id != ""',
        deleteRule: document.getElementById('newRuleDelete')?.value.trim() || '@request.auth.id != ""'
      };

      try {
        const res = await fetch('/api/collections', {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
            'Authorization': 'Bearer ' + (localStorage.getItem('padi_admin_token') || '')
          },
          body: JSON.stringify(payload)
        });
        const data = await res.json();
        if (res.ok) {
          const savedCol = data.item || data;
          cancelCreateTablePage();
          showToast(`Table "${savedCol.name}" created successfully!`);
          await loadCollections(savedCol.name);
          renderQuickMatrixTable();
        } else {
          if (alertEl) {
            alertEl.textContent = data.message || 'Failed to create table';
            alertEl.classList.remove('hidden');
          }
        }
      } catch (err) {
        if (alertEl) {
          alertEl.textContent = err.message;
          alertEl.classList.remove('hidden');
        }
      }
    }

    async function handleDeleteCurrentCollection() {
      if (currentCollection === 'users') return showToast('System table "users" cannot be deleted', 'error');
      if (!confirm(`Are you sure you want to delete table "${currentCollection}" and all its records?`)) return;

      const res = await fetch(`/api/collections/${currentCollection}`, { method: 'DELETE' });
      if (res.ok) {
        closeDrawer('collectionDrawer');
        localStorage.removeItem('padi_active_col');
        showToast(`Table "${currentCollection}" deleted`);
        await loadCollections('users');
        renderQuickMatrixTable();
      }
    }

    // ================= PADIBASE API EXPLORER PANE =================
    let currentSelectedApiTab = 'list';

    function openApiPreviewDrawer() {
      switchMainView('collections');
      switchWorkspaceSubView('api');
    }

    function openApiPreviewModal() {
      renderApiPane();
      const modal = document.getElementById('apiPreviewModal');
      if (modal) modal.classList.remove('hidden');
    }

    function closeApiPreviewModal() {
      const modal = document.getElementById('apiPreviewModal');
      if (modal) modal.classList.add('hidden');
    }

    function renderApiPane() {
      const col = collections.find(c => c.name === currentCollection);
      const colNameEl = document.getElementById('apiPreviewColName');
      if (colNameEl) colNameEl.textContent = currentCollection;
      const exportNameEl = document.getElementById('exportColBtnName');
      if (exportNameEl) exportNameEl.textContent = currentCollection;
      const isAuthCol = Boolean(col && col.type === 'auth');
      ['apiTabRegister', 'apiTabAuth', 'apiTabForgot', 'apiTabReset', 'apiTabChangePw', 'apiTabMe'].forEach(id => {
        const el = document.getElementById(id);
        if (el) el.classList.toggle('hidden', !isAuthCol);
      });
      const authGroupLabel = document.getElementById('apiAuthGroupLabel');
      if (authGroupLabel) authGroupLabel.classList.toggle('hidden', !isAuthCol);

      if (!isAuthCol && ['register', 'auth', 'forgot', 'reset', 'change_pw', 'me'].includes(currentSelectedApiTab)) {
        currentSelectedApiTab = 'list';
      }
      renderApiPreview(currentSelectedApiTab || 'list');
    }

    let currentApiPreviewUrl = "";
    let currentApiPreviewPath = "";
    let currentApiPreviewJs = "";
    let currentApiPreviewCurl = "";
    let currentApiPreviewResponseJson = "";
    let currentApiSnippetMode = "js"; // "js" | "curl"

    function toggleApiSnippetMode(mode) {
      currentApiSnippetMode = mode;
      const jsEl = document.getElementById('apiSnippetPre_js');
      const curlEl = document.getElementById('apiSnippetPre_curl');
      const btnJs = document.getElementById('apiSnippetBtn_js');
      const btnCurl = document.getElementById('apiSnippetBtn_curl');
      if (jsEl && curlEl) {
        jsEl.classList.toggle('hidden', mode !== 'js');
        curlEl.classList.toggle('hidden', mode !== 'curl');
      }
      if (btnJs && btnCurl) {
        btnJs.className = mode === 'js'
          ? "px-2 py-0.5 rounded-md bg-emerald-600 text-white font-bold text-[10px] shadow-2xs transition"
          : "px-2 py-0.5 rounded-md text-slate-600 hover:text-slate-900 font-semibold text-[10px] transition";
        btnCurl.className = mode === 'curl'
          ? "px-2 py-0.5 rounded-md bg-emerald-600 text-white font-bold text-[10px] shadow-2xs transition"
          : "px-2 py-0.5 rounded-md text-slate-600 hover:text-slate-900 font-semibold text-[10px] transition";
      }
    }

    function renderApiPreview(action) {
      currentSelectedApiTab = action;
      const baseUrl = window.location.origin;
      const col = collections.find(c => c.name === currentCollection) || { name: currentCollection, schema: [] };
      const cName = col.name;
      const content = document.getElementById('apiPreviewContent');

      document.querySelectorAll('#apiPreviewTabs button[data-api-tab]').forEach(btn => {
        const tabName = btn.getAttribute('data-api-tab');
        const isHidden = btn.classList.contains('hidden');
        if (tabName === action) {
          btn.className = `${isHidden ? 'hidden ' : ''}w-full text-left px-3 py-2 rounded-xl bg-emerald-600 text-white font-bold shadow-2xs flex items-center space-x-2 text-xs transition shrink-0`;
        } else if (['register', 'auth', 'forgot', 'reset', 'change_pw', 'me'].includes(tabName)) {
          btn.className = `${isHidden ? 'hidden ' : ''}w-full text-left px-3 py-2 rounded-xl text-teal-800 hover:bg-teal-50 font-semibold flex items-center space-x-2 text-xs transition shrink-0`;
        } else if (['transactions', 'sql'].includes(tabName)) {
          btn.className = `${isHidden ? 'hidden ' : ''}w-full text-left px-3 py-2 rounded-xl text-amber-800 hover:bg-amber-50 font-semibold flex items-center space-x-2 text-xs transition shrink-0`;
        } else {
          btn.className = `${isHidden ? 'hidden ' : ''}w-full text-left px-3 py-2 rounded-xl text-slate-700 hover:bg-slate-200/70 font-semibold flex items-center space-x-2 text-xs transition shrink-0`;
        }
      });

      const sampleObj = {};
      if (col.type === 'auth') {
        sampleObj.username = "john123";
        sampleObj.email = "john@example.com";
        sampleObj.password = "secret123456";
      }
      (col.schema || []).forEach(f => {
        sampleObj[f.name] = f.type === 'number' ? 100 : (f.type === 'bool' ? true : `sample_${f.name}`);
      });

      const sampleUserRecord = {
        id: "rec_u0192a8b",
        email: "john@example.com",
        username: "john123",
        name: "John Doe",
        role: "user",
        avatar: "",
        created: "2026-11-03 10:00:00",
        updated: "2026-11-03 10:00:00"
      };

      const sampleJwtToken = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJyZWNfdTAxOTJhOGIiLCJlbWFpbCI6ImpvaG5AZXhhbXBsZS5jb20iLCJyb2xlIjoidXNlciIsImV4cCI6MTgwMDAwMDAwMH0.signature";
      const sampleRecordItem = Object.assign({ id: "rec_001" }, sampleObj);
      delete sampleRecordItem.password;

      const docs = {
        list: {
          title: `Fetch Paginated Records (${cName})`,
          desc: `Retrieve ${cName} records with pagination, search filters, and sorting.`,
          method: 'GET',
          badge: 'bg-emerald-100 text-emerald-800',
          path: `/api/collections/${cName}/records?page=1&perPage=30`,
          statusCode: '200 OK',
          js: `const res = await fetch('${baseUrl}/api/collections/${cName}/records?page=1&perPage=30');\nconst { item, meta, debug } = await res.json();\nconsole.log(item, meta);`,
          curl: `curl -X GET "${baseUrl}/api/collections/${cName}/records?page=1&perPage=30"`
        },
        all: {
          title: `Fetch All Records Without Pagination (${cName})`,
          desc: `Retrieve rows from ${cName} via standard Padi Rust route (/api/${cName}/all). Capped at 5,000 max records to protect server memory. Optional query: ?limit=500 (max: 5000), ?offset=0, ?sort_by=id&order=desc.`,
          method: 'GET',
          badge: 'bg-emerald-100 text-emerald-800',
          path: `/api/collections/${cName}/records/all?limit=5000&sort_by=id&order=desc`,
          statusCode: '200 OK',
          js: `const res = await fetch('${baseUrl}/api/collections/${cName}/records/all?limit=5000&sort_by=id&order=desc');\nconst { item, debug } = await res.json();\nconsole.log(item);`,
          curl: `curl -X GET "${baseUrl}/api/collections/${cName}/records/all?limit=5000&sort_by=id&order=desc"`
        },
        view: {
          title: `Fetch Single Record by ID (${cName})`,
          desc: `Retrieve single record details from ${cName} using ID.`,
          method: 'GET',
          badge: 'bg-emerald-100 text-emerald-800',
          path: `/api/collections/${cName}/records/RECORD_ID`,
          statusCode: '200 OK',
          js: `const res = await fetch('${baseUrl}/api/collections/${cName}/records/RECORD_ID');\nconst { item } = await res.json();\nconsole.log(item);`,
          curl: `curl -X GET "${baseUrl}/api/collections/${cName}/records/RECORD_ID"`
        },
        create: {
          title: `Create New Record (${cName})`,
          desc: `Insert a new record into table ${cName}.`,
          method: 'POST',
          badge: 'bg-sky-100 text-sky-800',
          path: `/api/collections/${cName}/records`,
          statusCode: '201 Created',
          js: `const res = await fetch('${baseUrl}/api/collections/${cName}/records', {\n  method: 'POST',\n  headers: { 'Content-Type': 'application/json' },\n  body: JSON.stringify(${JSON.stringify(sampleObj, null, 2)})\n});\nconst { item } = await res.json();`,
          curl: `curl -X POST "${baseUrl}/api/collections/${cName}/records" \\\n  -H "Content-Type: application/json" \\\n  -d '${JSON.stringify(sampleObj, null, 2)}'`
        },
        update: {
          title: `Update Record (${cName})`,
          desc: `Modify some or all fields of an existing record by ID.`,
          method: 'PATCH',
          badge: 'bg-amber-100 text-amber-800',
          path: `/api/collections/${cName}/records/RECORD_ID`,
          statusCode: '200 OK',
          js: `const res = await fetch('${baseUrl}/api/collections/${cName}/records/RECORD_ID', {\n  method: 'PATCH',\n  headers: { 'Content-Type': 'application/json' },\n  body: JSON.stringify(${JSON.stringify(sampleObj, null, 2)})\n});\nconst { item } = await res.json();`,
          curl: `curl -X PATCH "${baseUrl}/api/collections/${cName}/records/RECORD_ID" \\\n  -H "Content-Type: application/json" \\\n  -d '${JSON.stringify(sampleObj, null, 2)}'`
        },
        delete: {
          title: `Delete Record (${cName})`,
          desc: `Remove a record from table ${cName} by ID.`,
          method: 'DELETE',
          badge: 'bg-rose-100 text-rose-800',
          path: `/api/collections/${cName}/records/RECORD_ID`,
          statusCode: '200 OK',
          js: `const res = await fetch('${baseUrl}/api/collections/${cName}/records/RECORD_ID', {\n  method: 'DELETE'\n});\nconst { item } = await res.json();`,
          curl: `curl -X DELETE "${baseUrl}/api/collections/${cName}/records/RECORD_ID"`
        },
        realtime: {
          title: `Subscribe to Realtime SSE Stream (${cName})`,
          desc: `Listen for live data changes when Realtime is enabled on table ${cName}.`,
          method: 'GET',
          badge: 'bg-purple-100 text-purple-800',
          path: `/api/realtime`,
          statusCode: '200 OK (text/event-stream)',
          js: `const evtSource = new EventSource('${baseUrl}/api/realtime');\nevtSource.onmessage = (event) => {\n  const data = JSON.parse(event.data);\n  if (data.collection === '${cName}') {\n    console.log('Change event:', data.action, data.record);\n  }\n};`,
          curl: `curl -N "${baseUrl}/api/realtime"`
        },
        file: {
          title: `Upload File / Image (${cName})`,
          desc: `Upload files via multipart/form-data and link them automatically to the record.`,
          method: 'POST',
          badge: 'bg-indigo-100 text-indigo-800',
          path: `/api/files/${cName}/RECORD_ID`,
          statusCode: '200 OK',
          js: `const formData = new FormData();\nformData.append('file', fileInputElement.files[0]);\n\nconst res = await fetch('${baseUrl}/api/files/${cName}/RECORD_ID', {\n  method: 'POST',\n  body: formData\n});\nconst result = await res.json();`,
          curl: `curl -X POST "${baseUrl}/api/files/${cName}/RECORD_ID" \\\n  -F "file=@/path/to/my_image.png"`
        },
        auth: {
          title: `User Login & Obtain JWT Token (${cName})`,
          desc: `Verify identity (username/email) & password and return a signed JWT authentication token along with user record.`,
          method: 'POST',
          badge: 'bg-teal-100 text-teal-800',
          path: `/api/collections/${cName}/auth-with-password`,
          statusCode: '200 OK',
          js: `const res = await fetch('${baseUrl}/api/collections/${cName}/auth-with-password', {\n  method: 'POST',\n  headers: { 'Content-Type': 'application/json' },\n  body: JSON.stringify({\n    identity: 'john@example.com',\n    password: 'secret123456'\n  })\n});\nconst { item } = await res.json();\nconsole.log(item.token, item.record);`,
          curl: `curl -X POST "${baseUrl}/api/collections/${cName}/auth-with-password" \\\n  -H "Content-Type: application/json" \\\n  -d '{\n  "identity": "john@example.com",\n  "password": "secret123456"\n}'`
        },
        register: {
          title: `Register New Account (${cName})`,
          desc: `Create a new user account in ${cName} and immediately receive a signed JWT token + user record.`,
          method: 'POST',
          badge: 'bg-teal-100 text-teal-800',
          path: `/api/collections/${cName}/register`,
          statusCode: '201 Created',
          js: `const res = await fetch('${baseUrl}/api/collections/${cName}/register', {\n  method: 'POST',\n  headers: { 'Content-Type': 'application/json' },\n  body: JSON.stringify({\n    email: 'john@example.com',\n    username: 'john123',\n    name: 'John Doe',\n    password: 'secret123456',\n    passwordConfirm: 'secret123456'\n  })\n});\nconst { item } = await res.json();\nconsole.log(item.token, item.record);`,
          curl: `curl -X POST "${baseUrl}/api/collections/${cName}/register" \\\n  -H "Content-Type: application/json" \\\n  -d '{\n  "email": "john@example.com",\n  "username": "john123",\n  "name": "John Doe",\n  "password": "secret123456",\n  "passwordConfirm": "secret123456"\n}'`
        },
        forgot: {
          title: `Forgot Password - Generate Reset Token (${cName})`,
          desc: `Generate a one-time password reset token for a user account by email or username.`,
          method: 'POST',
          badge: 'bg-teal-100 text-teal-800',
          path: `/api/collections/${cName}/forgot-password`,
          statusCode: '200 OK',
          js: `const res = await fetch('${baseUrl}/api/collections/${cName}/forgot-password', {\n  method: 'POST',\n  headers: { 'Content-Type': 'application/json' },\n  body: JSON.stringify({\n    email: 'john@example.com'\n  })\n});\nconst { item } = await res.json();\nconsole.log(item.email, item.reset_token);`,
          curl: `curl -X POST "${baseUrl}/api/collections/${cName}/forgot-password" \\\n  -H "Content-Type: application/json" \\\n  -d '{\n  "email": "john@example.com"\n}'`
        },
        reset: {
          title: `Reset Password with Token (${cName})`,
          desc: `Verify the one-time reset token and set a new Argon2id-hashed password for the account.`,
          method: 'POST',
          badge: 'bg-teal-100 text-teal-800',
          path: `/api/collections/${cName}/reset-password`,
          statusCode: '200 OK',
          js: `const res = await fetch('${baseUrl}/api/collections/${cName}/reset-password', {\n  method: 'POST',\n  headers: { 'Content-Type': 'application/json' },\n  body: JSON.stringify({\n    token: 'd7f4a1c2e8b940bf7c53e01a8',\n    password: 'newSecretPassword123',\n    passwordConfirm: 'newSecretPassword123'\n  })\n});\nconst { item } = await res.json();\nconsole.log(item.message);`,
          curl: `curl -X POST "${baseUrl}/api/collections/${cName}/reset-password" \\\n  -H "Content-Type: application/json" \\\n  -d '{\n  "token": "d7f4a1c2e8b940bf7c53e01a8",\n  "password": "newSecretPassword123",\n  "passwordConfirm": "newSecretPassword123"\n}'`
        },
        change_pw: {
          title: `Change Password for Logged-in User (${cName})`,
          desc: `Verify current password and update to a new password while returning a refreshed JWT token.`,
          method: 'POST',
          badge: 'bg-teal-100 text-teal-800',
          path: `/api/collections/${cName}/change-password`,
          statusCode: '200 OK',
          js: `const res = await fetch('${baseUrl}/api/collections/${cName}/change-password', {\n  method: 'POST',\n  headers: {\n    'Content-Type': 'application/json',\n    'Authorization': 'Bearer YOUR_JWT_TOKEN'\n  },\n  body: JSON.stringify({\n    oldPassword: 'secret123456',\n    newPassword: 'updatedPassword789',\n    passwordConfirm: 'updatedPassword789'\n  })\n});\nconst { item } = await res.json();\nconsole.log(item.token);`,
          curl: `curl -X POST "${baseUrl}/api/collections/${cName}/change-password" \\\n  -H "Content-Type: application/json" \\\n  -H "Authorization: Bearer YOUR_JWT_TOKEN" \\\n  -d '{\n  "oldPassword": "secret123456",\n  "newPassword": "updatedPassword789",\n  "passwordConfirm": "updatedPassword789"\n}'`
        },
        me: {
          title: `Get Current User Profile & Refresh JWT (${cName})`,
          desc: `Fetch the authenticated user's record and receive a refreshed JWT token.`,
          method: 'GET',
          badge: 'bg-teal-100 text-teal-800',
          path: `/api/collections/${cName}/me`,
          statusCode: '200 OK',
          js: `const res = await fetch('${baseUrl}/api/collections/${cName}/me', {\n  headers: {\n    'Authorization': 'Bearer YOUR_JWT_TOKEN'\n  }\n});\nconst { item } = await res.json();\nconsole.log(item.record, item.token);`,
          curl: `curl -X GET "${baseUrl}/api/collections/${cName}/me" \\\n  -H "Authorization: Bearer YOUR_JWT_TOKEN"`
        },
        transactions: {
          title: `Atomic Database Transaction / Batch (${cName})`,
          desc: `Execute multiple create, update, delete, or SQL operations inside a single atomic DB transaction (all-or-nothing rollback on failure).`,
          method: 'POST',
          badge: 'bg-amber-100 text-amber-800',
          path: `/api/transactions`,
          statusCode: '200 OK',
          js: `const res = await fetch('${baseUrl}/api/transactions', {\n  method: 'POST',\n  headers: {\n    'Content-Type': 'application/json',\n    'Authorization': 'Bearer YOUR_JWT_TOKEN'\n  },\n  body: JSON.stringify({\n    operations: [\n      {\n        action: 'create',\n        collection: '${cName}',\n        data: ${JSON.stringify(sampleObj)}\n      },\n      {\n        action: 'update',\n        collection: '${cName}',\n        id: 'rec_001',\n        data: ${JSON.stringify(sampleObj)}\n      }\n    ]\n  })\n});\nconst { item } = await res.json();`,
          curl: `curl -X POST "${baseUrl}/api/transactions" \\\n  -H "Content-Type: application/json" \\\n  -H "Authorization: Bearer YOUR_JWT_TOKEN" \\\n  -d '{\n  "operations": [\n    {\n      "action": "create",\n      "collection": "${cName}",\n      "data": ${JSON.stringify(sampleObj)}\n    }\n  ]\n}'`
        },
        sql: {
          title: `Execute Custom Parameterized SQL (${cName})`,
          desc: `Run custom SELECT (JOIN, GROUP BY, aggregations) or transactional SQL statements with bindings (Admin only).`,
          method: 'POST',
          badge: 'bg-amber-100 text-amber-800',
          path: `/api/sql`,
          statusCode: '200 OK',
          js: `const res = await fetch('${baseUrl}/api/sql', {\n  method: 'POST',\n  headers: {\n    'Content-Type': 'application/json',\n    'Authorization': 'Bearer ADMIN_JWT_TOKEN'\n  },\n  body: JSON.stringify({\n    sql: 'SELECT * FROM \`${cName}\` LIMIT ?',\n    params: [10],\n    transaction: false\n  })\n});\nconst { items } = await res.json();`,
          curl: `curl -X POST "${baseUrl}/api/sql" \\\n  -H "Content-Type: application/json" \\\n  -H "Authorization: Bearer ADMIN_JWT_TOKEN" \\\n  -d '{\n  "sql": "SELECT * FROM \\\`${cName}\\\` LIMIT ?",\n  "params": [10]\n}'`
        }
      };

      const d = docs[action] || docs.list;
      currentApiPreviewPath = d.path;
      currentApiPreviewUrl = `${baseUrl}${d.path}`;
      currentApiPreviewJs = d.js;
      currentApiPreviewCurl = d.curl;

      // Build specific, realistic Padi Rust Api Framework standard response for each action!
      let samplePadiResponse = null;

      switch (action) {
        case 'auth':
          samplePadiResponse = {
            message: "Success",
            message_code: "SUCCESS",
            success: true,
            item: {
              token: sampleJwtToken,
              record: sampleUserRecord
            },
            debug: {
              env: "development",
              database: activeDbDriver || "sqlite",
              execution_time: "0.55ms",
              queries_count: 1
            }
          };
          break;

        case 'register':
          samplePadiResponse = {
            message: "Success",
            message_code: "CREATED",
            success: true,
            item: {
              token: sampleJwtToken,
              record: sampleUserRecord
            },
            debug: {
              env: "development",
              database: activeDbDriver || "sqlite",
              execution_time: "0.85ms",
              queries_count: 2
            }
          };
          break;

        case 'forgot':
          samplePadiResponse = {
            message: "Password reset token generated successfully.",
            message_code: "SUCCESS",
            success: true,
            item: {
              email: "john@example.com",
              reset_token: "d7f4a1c2e8b940bf7c53e01a8"
            },
            debug: {
              env: "development",
              database: activeDbDriver || "sqlite",
              execution_time: "0.40ms",
              queries_count: 2
            }
          };
          break;

        case 'reset':
          samplePadiResponse = {
            message: "Password has been reset successfully.",
            message_code: "SUCCESS",
            success: true,
            item: {
              message: "Password has been reset successfully."
            },
            debug: {
              env: "development",
              database: activeDbDriver || "sqlite",
              execution_time: "14.20ms",
              queries_count: 2
            }
          };
          break;

        case 'change_pw':
          samplePadiResponse = {
            message: "Password updated successfully.",
            message_code: "SUCCESS",
            success: true,
            item: {
              token: sampleJwtToken
            },
            debug: {
              env: "development",
              database: activeDbDriver || "sqlite",
              execution_time: "15.10ms",
              queries_count: 2
            }
          };
          break;

        case 'me':
          samplePadiResponse = {
            message: "Success",
            message_code: "SUCCESS",
            success: true,
            item: {
              token: sampleJwtToken,
              record: sampleUserRecord
            },
            debug: {
              env: "development",
              database: activeDbDriver || "sqlite",
              execution_time: "0.38ms",
              queries_count: 1
            }
          };
          break;

        case 'list':
          samplePadiResponse = {
            message: "Success",
            message_code: "SUCCESS",
            success: true,
            item: [sampleRecordItem],
            meta: {
              current_page: 1,
              per_page: 30,
              total: 1,
              last_page: 1,
              from: 1,
              to: 1
            },
            debug: {
              env: "development",
              database: activeDbDriver || "sqlite",
              execution_time: "0.42ms",
              queries_count: 1
            }
          };
          break;

        case 'all':
          samplePadiResponse = {
            message: "Success",
            message_code: "SUCCESS",
            success: true,
            item: [sampleRecordItem],
            debug: {
              env: "development",
              database: activeDbDriver || "sqlite",
              execution_time: "0.38ms",
              queries_count: 1
            }
          };
          break;

        case 'view':
          samplePadiResponse = {
            message: "Success",
            message_code: "SUCCESS",
            success: true,
            item: sampleRecordItem,
            debug: {
              env: "development",
              database: activeDbDriver || "sqlite",
              execution_time: "0.28ms",
              queries_count: 1
            }
          };
          break;

        case 'create':
          samplePadiResponse = {
            message: "Success",
            message_code: "CREATED",
            success: true,
            item: sampleRecordItem,
            debug: {
              env: "development",
              database: activeDbDriver || "sqlite",
              execution_time: "0.75ms",
              queries_count: 1
            }
          };
          break;

        case 'update':
          samplePadiResponse = {
            message: "Success",
            message_code: "SUCCESS",
            success: true,
            item: sampleRecordItem,
            debug: {
              env: "development",
              database: activeDbDriver || "sqlite",
              execution_time: "0.62ms",
              queries_count: 2
            }
          };
          break;

        case 'delete':
          samplePadiResponse = {
            message: "Record deleted successfully.",
            message_code: "SUCCESS",
            success: true,
            item: {
              id: "rec_001",
              deleted: true
            },
            debug: {
              env: "development",
              database: activeDbDriver || "sqlite",
              execution_time: "0.45ms",
              queries_count: 1
            }
          };
          break;

        case 'file':
          samplePadiResponse = {
            message: "File(s) uploaded successfully.",
            message_code: "SUCCESS",
            success: true,
            item: {
              uploaded: [
                {
                  field: "avatar",
                  filename: "f1a2b3c4_photo.png",
                  original_name: "photo.png",
                  size: 184320,
                  mime: "image/png",
                  url: `/api/files/${cName}/rec_001/f1a2b3c4_photo.png`
                }
              ],
              record: Object.assign({}, sampleRecordItem, { avatar: "f1a2b3c4_photo.png" })
            },
            debug: {
              env: "development",
              database: activeDbDriver || "sqlite",
              execution_time: "1.20ms",
              queries_count: 2
            }
          };
          break;

        case 'realtime':
          samplePadiResponse = {
            action: "create",
            collection: cName,
            record: sampleRecordItem
          };
          break;

        case 'transactions':
          samplePadiResponse = {
            message: "Transaction committed successfully.",
            message_code: "SUCCESS",
            success: true,
            item: {
              committed: true,
              operations_count: 2,
              results: [
                { action: "create", collection: cName, id: "rec_001", success: true },
                { action: "update", collection: cName, id: "rec_002", success: true }
              ]
            },
            debug: {
              env: "development",
              database: activeDbDriver || "sqlite",
              execution_time: "1.85ms",
              queries_count: 3
            }
          };
          break;

        case 'sql':
          samplePadiResponse = {
            message: "SQL executed successfully.",
            message_code: "SUCCESS",
            success: true,
            item: [
              { id: 1, name: "Sample Item", total_records: 10 }
            ],
            debug: {
              env: "development",
              database: activeDbDriver || "sqlite",
              execution_time: "0.52ms",
              queries_count: 1
            }
          };
          break;

        default:
          samplePadiResponse = {
            message: "Success",
            message_code: "SUCCESS",
            success: true,
            item: sampleRecordItem,
            debug: {
              env: "development",
              database: activeDbDriver || "sqlite",
              execution_time: "0.45ms"
            }
          };
      }

      currentApiPreviewResponseJson = JSON.stringify(samplePadiResponse, null, 2);

      content.innerHTML = `
        <div>
          <h4 class="text-sm font-extrabold text-slate-900">${d.title}</h4>
          <p class="text-slate-500 mt-0.5">${d.desc}</p>
        </div>
        <div class="flex flex-wrap items-center justify-between bg-emerald-50/60 p-3 rounded-2xl font-mono gap-2 border border-emerald-200/80">
          <div class="flex items-center space-x-2.5 min-w-0">
            <span class="px-2.5 py-1 rounded-lg font-extrabold text-[10px] shrink-0 ${d.badge}">${d.method}</span>
            <span class="text-slate-900 font-bold break-all text-xs">${d.path}</span>
          </div>
          <div class="flex items-center space-x-2 shrink-0">
            <button type="button" onclick="copyToClipboard(currentApiPreviewPath)" class="px-2.5 py-1 rounded-lg bg-white hover:bg-slate-100 text-slate-700 border border-slate-200 text-[11px] font-sans font-bold flex items-center space-x-1 transition cursor-pointer">
              <i class="fa-regular fa-copy text-[10px]"></i>
              <span>Copy Path</span>
            </button>
            <button type="button" onclick="copyToClipboard(currentApiPreviewUrl)" class="px-2.5 py-1 rounded-lg padi-btn-primary text-white text-[11px] font-sans font-bold flex items-center space-x-1 transition cursor-pointer">
              <i class="fa-solid fa-link text-[10px]"></i>
              <span>Copy URL</span>
            </button>
          </div>
        </div>
        <div class="grid grid-cols-1 lg:grid-cols-2 gap-4">
          <div>
            <div class="flex items-center justify-between mb-1.5">
              <div class="flex items-center space-x-1.5 bg-slate-200/60 p-0.5 rounded-lg border border-slate-200">
                <button type="button" id="apiSnippetBtn_js" onclick="toggleApiSnippetMode('js')" class="px-2 py-0.5 rounded-md ${currentApiSnippetMode === 'js' ? 'bg-emerald-600 text-white font-bold shadow-2xs' : 'text-slate-600 hover:text-slate-900 font-semibold'} text-[10px] transition cursor-pointer">
                  JS Fetch
                </button>
                <button type="button" id="apiSnippetBtn_curl" onclick="toggleApiSnippetMode('curl')" class="px-2 py-0.5 rounded-md ${currentApiSnippetMode === 'curl' ? 'bg-emerald-600 text-white font-bold shadow-2xs' : 'text-slate-600 hover:text-slate-900 font-semibold'} text-[10px] transition cursor-pointer">
                  cURL
                </button>
              </div>
              <button type="button" onclick="copyToClipboard(currentApiSnippetMode === 'js' ? currentApiPreviewJs : currentApiPreviewCurl)" class="px-2.5 py-1 rounded-lg border border-slate-200 bg-white hover:bg-slate-50 text-slate-700 text-[10px] font-bold flex items-center space-x-1 transition cursor-pointer">
                <i class="fa-regular fa-copy text-[9px]"></i>
                <span>Copy Snippet</span>
              </button>
            </div>
            <pre id="apiSnippetPre_js" class="${currentApiSnippetMode === 'js' ? '' : 'hidden '}bg-slate-950 text-emerald-400 p-4 rounded-2xl font-mono text-[11px] overflow-x-auto min-h-[220px] max-h-[380px]">${d.js}</pre>
            <pre id="apiSnippetPre_curl" class="${currentApiSnippetMode === 'curl' ? '' : 'hidden '}bg-slate-950 text-emerald-400 p-4 rounded-2xl font-mono text-[11px] overflow-x-auto min-h-[220px] max-h-[380px]">${d.curl}</pre>
          </div>
          <div>
            <div class="flex items-center justify-between mb-1.5">
              <div class="text-[11px] font-extrabold uppercase tracking-wider text-teal-700 flex items-center space-x-1.5">
                <i class="fa-solid fa-code text-[10px]"></i>
                <span>JSON Response (Padi Framework)</span>
              </div>
              <div class="flex items-center space-x-1.5">
                <span class="text-[10px] font-mono text-slate-500 font-bold mr-1">${d.statusCode || '200 OK'}</span>
                <button type="button" onclick="copyToClipboard(currentApiPreviewResponseJson)" class="px-2 py-1 rounded-lg border border-slate-200 bg-white hover:bg-slate-50 text-slate-700 text-[10px] font-bold flex items-center space-x-1 transition cursor-pointer">
                  <i class="fa-regular fa-copy text-[9px]"></i>
                  <span>Copy</span>
                </button>
              </div>
            </div>
            <pre class="bg-slate-950 text-teal-300 p-4 rounded-2xl font-mono text-[11px] overflow-x-auto min-h-[220px] max-h-[380px]">${currentApiPreviewResponseJson}</pre>
          </div>
        </div>
      `;
    }

    // ================= SETTINGS, BACKUPS, ADMINS, EXPORT/IMPORT =================
    async function loadAppSettings() {
      try {
        const res = await fetch('/api/settings');
        const data = await res.json();
        const settings = data.item || data;
        if (settings.roles && Array.isArray(settings.roles)) {
          AVAILABLE_ROLES = settings.roles.map(r => String(r).trim().toLowerCase()).filter(Boolean);
        }
        renderRolesSettings();
        if (settings.meta) {
          const appName = settings.meta.appName || 'PadiBase Studio';
          document.getElementById('settingAppName').value = appName;
          document.getElementById('settingAppUrl').value = settings.meta.appUrl || window.location.origin;
          const hc = document.getElementById('settingHideControls');
          if (hc) hc.checked = Boolean(settings.meta.hideControls);
          const headerTitle = document.getElementById('headerAppName');
          if (headerTitle) headerTitle.textContent = appName;
        }
      } catch (_) {}
    }

    function openSettingsRoles() {
      switchMainView('settings');
      switchSettingsTab('roles');
    }

    function renderRolesSettings() {
      const container = document.getElementById('settingsRolesContainer');
      const countBadge = document.getElementById('settingsRolesCountBadge');
      if (countBadge) countBadge.textContent = `${AVAILABLE_ROLES.length} active roles`;
      if (!container) return;

      container.innerHTML = AVAILABLE_ROLES.map(role => {
        const isCore = role === 'admin' || role === 'user';
        return `
          <div class="inline-flex items-center space-x-1.5 px-3 py-1.5 rounded-xl bg-white border border-slate-200 text-xs font-mono font-bold text-slate-800 shadow-2xs hover:border-emerald-300 transition">
            <i class="fa-solid fa-shield-halved text-[10px] text-emerald-600"></i>
            <span>${role}</span>
            ${!isCore ? `
              <button type="button" onclick="deleteRoleFromSettings('${role}')" class="ml-1 w-4 h-4 rounded-full hover:bg-rose-100 text-slate-400 hover:text-rose-600 inline-flex items-center justify-center transition cursor-pointer" title="Delete role '${role}'">
                <i class="fa-solid fa-xmark text-[10px]"></i>
              </button>
            ` : `
              <span class="text-[9px] px-1.5 py-0.2 rounded bg-slate-100 text-slate-400 font-sans font-normal ml-1" title="Core System Role">core</span>
            `}
          </div>
        `;
      }).join('');
    }

    async function addRoleToSettings() {
      const inp = document.getElementById('newRoleInput');
      if (!inp) return;
      const val = inp.value.trim().toLowerCase();
      if (!val) {
        showToast('Please enter a role name', 'error');
        return;
      }
      if (!/^[a-z0-9_-]+$/.test(val)) {
        showToast('Role name must only contain lowercase letters, numbers, hyphens, or underscores', 'error');
        return;
      }
      if (AVAILABLE_ROLES.includes(val)) {
        showToast(`Role "${val}" already exists`, 'error');
        return;
      }
      AVAILABLE_ROLES.push(val);
      inp.value = '';
      await saveRolesToSettings();
    }

    async function deleteRoleFromSettings(role) {
      if (role === 'admin' || role === 'user') {
        showToast('Core system roles (admin, user) cannot be deleted', 'error');
        return;
      }
      if (!confirm(`Delete role "${role}"? Records created or updated with this role will be rejected.`)) return;
      AVAILABLE_ROLES = AVAILABLE_ROLES.filter(r => r !== role);
      await saveRolesToSettings();
    }

    async function saveRolesToSettings() {
      try {
        const res = await fetch('/api/settings', {
          method: 'PATCH',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ roles: AVAILABLE_ROLES })
        });
        if (res.ok) {
          renderRolesSettings();
          showToast(`Roles updated successfully (${AVAILABLE_ROLES.length} roles available)`);
        } else {
          const d = await res.json();
          showToast(d.message || 'Failed to save roles', 'error');
        }
      } catch (e) {
        showToast(e.message, 'error');
      }
    }

    async function saveAppSettings() {
      const appName = document.getElementById('settingAppName').value.trim();
      const appUrl = document.getElementById('settingAppUrl').value.trim();
      const hideControls = Boolean(document.getElementById('settingHideControls')?.checked);
      const res = await fetch('/api/settings', {
        method: 'PATCH',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ meta: { appName, appUrl, hideControls } })
      });
      if (res.ok) {
        const headerTitle = document.getElementById('headerAppName');
        if (headerTitle && appName) headerTitle.textContent = appName;
        showToast('Application configuration saved successfully!');
      }
    }

    async function loadBackups() {
      const tbody = document.getElementById('backupsTableBody');
      try {
        const res = await fetch('/api/backups');
        const data = await res.json();
        const items = data.item || data.items || [];
        if (items.length === 0) {
          tbody.innerHTML = `<tr><td colspan="4" class="py-8 text-center text-slate-400 font-sans">Belum ada file backup database. Klik "Create Backup Now" untuk membuat cadangan.</td></tr>`;
        } else {
          tbody.innerHTML = items.map(b => `
            <tr class="hover:bg-emerald-50/30">
              <td class="py-3 px-4 font-bold text-slate-800"><i class="fa-solid fa-box-archive text-emerald-600 mr-2"></i>${b.key}</td>
              <td class="py-3 px-4 text-slate-500">${formatBytes(b.size)}</td>
              <td class="py-3 px-4 text-slate-400">${formatDateCell(b.created)}</td>
              <td class="py-3 px-4 text-right space-x-2">
                <a href="/api/backups/${b.key}" download class="text-emerald-600 hover:underline font-sans font-bold">Download</a>
                <button onclick="handleRestoreBackup('${b.key}')" class="text-amber-600 hover:underline font-sans font-bold ml-2">Restore</button>
                <button onclick="handleDeleteBackup('${b.key}')" class="text-rose-600 hover:underline font-sans font-bold ml-2">Delete</button>
              </td>
            </tr>
          `).join('');
        }
      } catch (_) {}
    }

    async function handleCreateBackup() {
      const res = await fetch('/api/backups', { method: 'POST' });
      if (res.ok) {
        showToast('Database backup created successfully');
        loadBackups();
      }
    }

    async function handleRestoreBackup(key) {
      if (!confirm(`Restore database from "${key}"? Current data will be replaced by backup contents.`)) return;
      const res = await fetch(`/api/backups/${key}/restore`, { method: 'POST' });
      if (res.ok) {
        showToast('Database restored successfully');
        loadCollections();
      }
    }

    async function handleDeleteBackup(key) {
      if (!confirm(`Delete backup file "${key}"?`)) return;
      const res = await fetch(`/api/backups/${key}`, { method: 'DELETE' });
      if (res.ok) {
        showToast('Backup file deleted');
        loadBackups();
      }
    }

    function copyExportedSchema() {
      navigator.clipboard.writeText(JSON.stringify(collections, null, 2));
      showToast('JSON schema copied to clipboard');
    }

    function downloadExportedSchema() {
      const blob = new Blob([JSON.stringify(collections, null, 2)], { type: 'application/json' });
      const a = document.createElement('a');
      a.href = URL.createObjectURL(blob);
      a.download = 'padibase_schema_blueprint.json';
      a.click();
    }

    async function submitImportCollectionsSchema() {
      try {
        const parsed = JSON.parse(document.getElementById('importSchemaTextarea').value || '[]');
        const cols = Array.isArray(parsed) ? parsed : [parsed];
        const res = await fetch('/api/collections/import', {
          method: 'PUT',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ collections: cols })
        });
        if (res.ok) {
          showToast('Schema blueprint successfully applied!');
          await loadCollections();
          switchMainView('collections');
        }
      } catch (e) {
        showToast('Invalid JSON format: ' + e.message, 'error');
      }
    }

    async function loadAdminAccounts() {
      const tbody = document.getElementById('adminTableBody');
      const res = await fetch('/api/admins');
      const data = await res.json();
      const items = data.item || data.items || [];
      tbody.innerHTML = items.map(a => `
        <tr class="hover:bg-emerald-50/30">
          <td class="py-3 px-4 font-semibold text-slate-700">${a.id}</td>
          <td class="py-3 px-4 text-slate-900 font-bold">${a.email}</td>
          <td class="py-3 px-4 text-slate-400">${formatDateCell(a.created)}</td>
          <td class="py-3 px-4 text-right">
            <button onclick="handleDeleteAdmin('${a.id}')" class="text-rose-600 hover:underline font-sans font-bold">Delete</button>
          </td>
        </tr>
      `).join('');
    }

    async function handleCreateAdditionalAdmin() {
      const email = document.getElementById('newAdminEmail').value.trim();
      const password = document.getElementById('newAdminPassword').value;
      if (!email || !password) return showToast('Email and password are required', 'error');
      const res = await fetch('/api/admins', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ email, password })
      });
      if (res.ok) {
        document.getElementById('newAdminEmail').value = '';
        document.getElementById('newAdminPassword').value = '';
        showToast('New superuser added successfully');
        loadAdminAccounts();
      } else {
        const d = await res.json();
        showToast(d.message || 'Failed to add admin', 'error');
      }
    }

    async function handleDeleteAdmin(id) {
      if (!confirm('Delete this superuser account?')) return;
      const res = await fetch(`/api/admins/${id}`, { method: 'DELETE' });
      if (res.ok) {
        showToast('Superuser deleted');
        loadAdminAccounts();
      } else {
        const d = await res.json();
        showToast(d.message || 'Cannot delete the last admin account', 'error');
      }
    }

    // ================= DATA EXPORT & IMPORT =================
    function handleExportCollection() {
      const a = document.createElement('a');
      a.href = `/api/collections/${currentCollection}/export`;
      a.download = `${currentCollection}_export.json`;
      a.click();
    }

    function handleExportApiCollection(isFull = false) {
      const a = document.createElement('a');
      if (isFull) {
        a.href = `/api/api-collection?download=true`;
        a.download = `padi_base_api_collection.json`;
      } else {
        a.href = `/api/collections/${currentCollection}/api-collection?download=true`;
        a.download = `${currentCollection}_collection.json`;
      }
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      showToast(isFull ? 'Successfully downloaded full API collection (.json)' : `Successfully downloaded API collection for ${currentCollection} (.json)`);
    }

    function openImportModal() {
      document.getElementById('importColNameBadge').textContent = currentCollection;
      document.getElementById('importJsonTextarea').value = '';
      document.getElementById('importRecordsModal').classList.remove('hidden');
    }

    function handleImportFileSelect(event) {
      const file = event.target.files[0];
      if (!file) return;
      const reader = new FileReader();
      reader.onload = (e) => { document.getElementById('importJsonTextarea').value = e.target.result; };
      reader.readAsText(file);
    }

    async function submitImportRecords() {
      try {
        let payload = JSON.parse(document.getElementById('importJsonTextarea').value || '[]');
        if (!Array.isArray(payload)) payload = [payload];
        const res = await fetch(`/api/collections/${currentCollection}/import`, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify(payload)
        });
        const data = await res.json();
        if (res.ok) {
          closeDrawer('importRecordsModal');
          const count = data.item?.imported ?? data.imported ?? payload.length;
          showToast(`Successfully imported ${count} record(s)`);
          loadRecords();
        }
      } catch (e) {
        showToast('Invalid JSON: ' + e.message, 'error');
      }
    }

    // ================= REALTIME SSE =================
    function initSSE() {
      if (sseSource) sseSource.close();
      sseSource = new EventSource('/api/realtime');
      sseSource.onmessage = (event) => {
        try {
          const data = JSON.parse(event.data);
          logSSEEvent(data);
          if (data.collection === currentCollection) loadRecords();
        } catch (_) {}
      };
    }

    let realtimeFormatPretty = true;

    function toggleRealtimeFormat() {
      realtimeFormatPretty = !realtimeFormatPretty;
      const lbl = document.getElementById('realtimeFormatLabel');
      if (lbl) lbl.textContent = realtimeFormatPretty ? 'Pretty JSON' : 'Compact JSON';
      document.querySelectorAll('#eventLogContainer [data-realtime-record]').forEach(el => {
        try {
          const raw = el.getAttribute('data-realtime-record');
          const obj = JSON.parse(decodeURIComponent(raw));
          const jsonStr = realtimeFormatPretty ? JSON.stringify(obj, null, 2) : JSON.stringify(obj);
          el.innerHTML = syntaxHighlightJson(jsonStr);
        } catch (_) {}
      });
    }

    function syntaxHighlightJson(jsonStr) {
      if (!jsonStr) return '';
      const escaped = jsonStr
        .replace(/&/g, '&amp;')
        .replace(/</g, '&lt;')
        .replace(/>/g, '&gt;');
      return escaped.replace(/("(\\u[a-zA-Z0-9]{4}|\\[^u]|[^\\"])*"(\s*:)?|\b(true|false|null)\b|-?\d+(?:\.\d*)?(?:[eE][+\-]?\d+)?)/g, function (match) {
        let cls = 'text-amber-300';
        if (/^"/.test(match)) {
          if (/:$/.test(match)) {
            cls = 'text-teal-400 font-bold';
          } else {
            cls = 'text-emerald-300';
          }
        } else if (/true|false/.test(match)) {
          cls = 'text-sky-400 font-semibold';
        } else if (/null/.test(match)) {
          cls = 'text-slate-500 italic';
        }
        return '<span class="' + cls + '">' + match + '</span>';
      });
    }

    function logSSEEvent(data) {
      const container = document.getElementById('eventLogContainer');
      const action = (data.action || 'event').toUpperCase();
      let badgeCls = 'text-emerald-400 bg-emerald-950/80 border-emerald-800/60';
      if (data.action === 'update') badgeCls = 'text-amber-400 bg-amber-950/80 border-amber-800/60';
      if (data.action === 'delete') badgeCls = 'text-rose-400 bg-rose-950/80 border-rose-800/60';

      const jsonStr = realtimeFormatPretty ? JSON.stringify(data.record, null, 2) : JSON.stringify(data.record);
      const highlighted = syntaxHighlightJson(jsonStr);
      const encodedRecord = encodeURIComponent(JSON.stringify(data.record || {}));
      const recordId = data.record && (data.record.id !== undefined && data.record.id !== null) ? data.record.id : '';

      const el = document.createElement('div');
      el.className = "border border-slate-800/80 bg-slate-900/60 rounded-xl p-3 sm:p-3.5 space-y-2.5 hover:border-slate-700 transition shadow-xs";
      el.innerHTML = `
        <div class="flex items-center justify-between gap-2 flex-wrap">
          <div class="flex items-center space-x-2 shrink-0 flex-wrap gap-y-1">
            <span class="text-slate-500 font-mono text-xs whitespace-nowrap shrink-0">[${new Date().toLocaleTimeString()}]</span>
            <span class="font-extrabold uppercase px-2 py-0.5 rounded text-[11px] border ${badgeCls} shrink-0">${action}</span>
            <span class="text-teal-300 font-bold text-xs font-mono shrink-0">${data.collection}</span>
            ${recordId !== '' ? `<span class="text-slate-400 text-xs font-mono bg-slate-800/80 px-1.5 py-0.5 rounded shrink-0">#${recordId}</span>` : ''}
          </div>
          <div class="flex items-center space-x-2 shrink-0">
            <button type="button" onclick="copyRealtimePayload(this)" class="px-2.5 py-1 rounded-lg bg-slate-800 hover:bg-slate-700 text-slate-300 hover:text-white text-xs font-bold transition flex items-center space-x-1.5 border border-slate-700/80">
              <i class="fa-regular fa-copy text-[11px]"></i>
              <span>Copy JSON</span>
            </button>
          </div>
        </div>
        <div class="bg-slate-950/90 rounded-lg p-3 border border-slate-900/90 overflow-x-auto">
          <pre class="text-slate-200 text-xs font-mono leading-relaxed whitespace-pre-wrap break-all select-all" data-realtime-record="${encodedRecord}">${highlighted}</pre>
        </div>
      `;
      container.prepend(el);
    }

    function copyRealtimePayload(btn) {
      const card = btn.closest('.border');
      if (!card) return;
      const pre = card.querySelector('[data-realtime-record]');
      if (!pre) return;
      try {
        const raw = decodeURIComponent(pre.getAttribute('data-realtime-record'));
        const obj = JSON.parse(raw);
        navigator.clipboard.writeText(JSON.stringify(obj, null, 2));
        showToast('JSON copied to clipboard', 'success');
      } catch (_) {
        navigator.clipboard.writeText(pre.textContent);
        showToast('Data copied to clipboard', 'success');
      }
    }

    function clearEventLogs() {
      const container = document.getElementById('eventLogContainer');
      if (container) {
        container.innerHTML = '<div class="text-emerald-400/80">🌾 PadiBase Realtime monitor ready — live mutations on tables with Realtime enabled will appear here...</div>';
      }
    }

    function closeDrawer(id) {
      if (id === 'recordDrawer' || id === 'recordDrawerModal') return closeRecordEditorPage();
      if (id === 'collectionDrawer' || id === 'newCollectionModal') return cancelCreateTablePage();
      if (id === 'collectionSettingsModal') return closeSchemaEditorPage();
      if (id === 'apiPreviewModal') return closeApiPreviewModal();
      const el = document.getElementById(id);
      if (el) el.classList.add('hidden');
    }

    function formatBytes(bytes) {
      if (!bytes) return '0 B';
      const k = 1024, sizes = ['B', 'KB', 'MB', 'GB'];
      const i = Math.floor(Math.log(bytes) / Math.log(k));
      return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
    }

    document.addEventListener('keydown', (e) => {
      if (e.key === 'Escape') {
        closeRecordEditorPage();
        closeSchemaEditorPage();
        cancelCreateTablePage();
        closeApiPreviewModal();
        closeDrawer('importRecordsModal');
        closeDrawer('quickGuideModal');
        closeDrawer('imageLightboxModal');
      }
    });

    window.onload = init;
