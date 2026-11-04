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
      const res = await _originalFetch(url, opts);
      // A 401 on an authenticated dashboard call means the admin session was revoked or expired.
      if (res.status === 401 && token && typeof url === 'string' && url.startsWith('/api/') && !url.includes('auth-with-password')) {
        localStorage.removeItem('padi_admin_token');
        localStorage.removeItem('padi_admin_email');
        location.reload();
      }
      return res;
    };

    // Query-string token for resources the browser loads itself (<img>, EventSource).
    function fileTokenQuery() {
      const t = localStorage.getItem('padi_admin_token');
      return t ? '?token=' + encodeURIComponent(t) : '';
    }

    // Authenticated backup download (a plain <a href> cannot send the Authorization header).
    async function downloadBackup(key) {
      try {
        const res = await fetch('/api/backups/' + encodeURIComponent(key));
        if (!res.ok) throw new Error('Download failed (' + res.status + ')');
        const blob = await res.blob();
        const a = document.createElement('a');
        a.href = URL.createObjectURL(blob);
        a.download = key;
        document.body.appendChild(a);
        a.click();
        a.remove();
        setTimeout(() => URL.revokeObjectURL(a.href), 10000);
      } catch (e) {
        showToast(e.message, 'error');
      }
    }

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
      const views = ['collections', 'endpoints', 'logs', 'settings', 'sqlbuilder'];
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
        loadAppSettings();
        loadBackups();
        loadAdminAccounts();
        loadCronJobs();
      } else if (view === 'sqlbuilder') {
        initQueryBuilder();
      } else if (view === 'logs') {
        updateRealtimeTableOptions();
        if (!sseSource || sseSource.readyState === EventSource.CLOSED) {
          initSSE();
        }
      }
    }

    // ================= DYNAMIC CUSTOM SQL ENDPOINTS BUILDER =================
    let customEndpoints = [];
    let currentCustomEpId = null;

    function isOverrideEndpointPath(rawPath) {
      if (!rawPath) return false;
      const clean = rawPath.trim().replace(/^\/+/, '').toLowerCase();

      // Core built-in system routes
      if (
        clean === 'api/health' ||
        clean === 'api/tables' ||
        clean === 'api/collections' ||
        clean === 'api/realtime' ||
        clean === 'api/endpoints' ||
        clean === 'api/settings' ||
        clean === 'api/backups'
      ) {
        return true;
      }

      // Built-in table default routes (e.g. api/banks, api/banks/:id, banks)
      for (const c of (collections || [])) {
        const colName = (c.name || '').toLowerCase();
        if (!colName) continue;

        // Exact matches with or without /api
        if (clean === `api/${colName}` || clean === colName) {
          return true;
        }

        // Sub-routes with or without /api
        if (
          clean.startsWith(`api/${colName}/`) ||
          clean.startsWith(`${colName}/`) ||
          clean.startsWith(`api/tables/${colName}/`) ||
          clean.startsWith(`api/collections/${colName}/`)
        ) {
          return true;
        }
      }

      return false;
    }

    let lastSelectedPrefix = '';
    let originalEndpointPath = '';

    function getSavedCustomPrefixes() {
      try {
        const raw = localStorage.getItem('padibase_custom_prefixes');
        if (raw) {
          const arr = JSON.parse(raw);
          if (Array.isArray(arr)) {
            return arr.filter(p => typeof p === 'string' && p.trim() && !['v1/', 'custom/', 'tables/', 'collections/'].includes(p.trim()));
          }
        }
      } catch (e) {
        console.error('Failed to load saved prefixes:', e);
      }
      return [];
    }

    function saveCustomPrefix(newPrefix) {
      if (!newPrefix) return null;
      let p = newPrefix.trim();
      if (!p) return null;

      if (p.startsWith('/api/')) {
        p = p.substring('/api/'.length);
      } else if (p.startsWith('api/')) {
        p = p.substring('api/'.length);
      }

      if (!p.endsWith('/')) {
        p += '/';
      }

      if (p === '/' || p === '' || p === '__custom__') return null;

      const list = getSavedCustomPrefixes();
      if (!list.includes(p)) {
        list.push(p);
        try {
          localStorage.setItem('padibase_custom_prefixes', JSON.stringify(list));
        } catch (e) {
          console.error('Failed to save custom prefix:', e);
        }
      }
      renderPrefixDropdownOptions(p);
      return p;
    }

    function deleteCustomPrefix(prefixToDelete) {
      if (!prefixToDelete) return;
      let list = getSavedCustomPrefixes();
      list = list.filter(p => p !== prefixToDelete);
      try {
        localStorage.setItem('padibase_custom_prefixes', JSON.stringify(list));
      } catch (e) {
        console.error('Failed to update saved prefixes:', e);
      }
      renderPrefixDropdownOptions('');
      updateCustomEpUrlPreview();
    }

    function deleteCurrentSelectedPrefix() {
      const sel = document.getElementById('epPrefixSelect');
      if (!sel) return;
      const val = sel.value;
      if (!val || val === '/' || val === '__custom__') return;
      if (!confirm(`Delete saved prefix "${val}"?`)) return;
      deleteCustomPrefix(val);
      if (typeof showToast === 'function') {
        showToast(`Prefix "${val}" deleted.`);
      }
    }

    function saveNewCustomPrefixFromInput() {
      const input = document.getElementById('epCustomPrefixInput');
      if (!input) return;
      const raw = input.value.trim();
      if (!raw) {
        if (typeof showToast === 'function') {
          showToast('Please enter a prefix first', 'error');
        }
        return;
      }
      const saved = saveCustomPrefix(raw);
      if (saved) {
        input.value = '';
        lastSelectedPrefix = saved;
        if (typeof showToast === 'function') {
          showToast(`Prefix "${saved}" saved to dropdown!`);
        }
      }
    }

    function getEndpointPathMeta(epId) {
      if (!epId) return null;
      try {
        const raw = localStorage.getItem('padibase_endpoint_paths_meta');
        if (raw) {
          const map = JSON.parse(raw);
          return map[epId] || null;
        }
      } catch (e) {
        console.error('Failed to read endpoint path meta:', e);
      }
      return null;
    }

    function saveEndpointPathMeta(epId, prefix, subPath) {
      if (!epId) return;
      try {
        const raw = localStorage.getItem('padibase_endpoint_paths_meta');
        const map = raw ? JSON.parse(raw) : {};
        map[epId] = { prefix: prefix || '', subPath: subPath || '' };
        localStorage.setItem('padibase_endpoint_paths_meta', JSON.stringify(map));
      } catch (e) {
        console.error('Failed to save endpoint path meta:', e);
      }
    }

    function deleteEndpointPathMeta(epId) {
      if (!epId) return;
      try {
        const raw = localStorage.getItem('padibase_endpoint_paths_meta');
        if (raw) {
          const map = JSON.parse(raw);
          delete map[epId];
          localStorage.setItem('padibase_endpoint_paths_meta', JSON.stringify(map));
        }
      } catch (e) {
        console.error('Failed to delete endpoint path meta:', e);
      }
    }

    function parseEndpointPathAndPrefix(ep) {
      const fullPath = (ep.path || '').trim().replace(/^\/+/, '');
      if (!fullPath) {
        return { prefix: '', subPath: '' };
      }

      // 1. Check saved meta first
      const meta = getEndpointPathMeta(ep.id);
      if (meta && (meta.prefix !== undefined || meta.subPath !== undefined)) {
        const cleanSub = (meta.subPath || '').trim().replace(/^\/+/, '');
        const pfx = meta.prefix || '';
        let testCombined = '';
        if (pfx === '/') {
          testCombined = cleanSub;
        } else if (pfx.startsWith('/') && !pfx.startsWith('/api')) {
          testCombined = `${pfx.replace(/^\/+/, '')}${cleanSub}`;
        } else {
          const subPfx = pfx.replace(/^\/?api\/?/, '').replace(/^\/+/, '');
          testCombined = subPfx ? `api/${subPfx}${cleanSub}` : `api/${cleanSub}`;
        }
        if (testCombined === fullPath) {
          return { prefix: meta.prefix, subPath: cleanSub };
        }
      }

      // 2. Check if it matches any saved custom prefixes from localStorage
      const savedList = getSavedCustomPrefixes();
      for (const sp of savedList) {
        const cleanSp = sp.replace(/^\/+/, '');
        if (sp.startsWith('/') && !sp.startsWith('/api')) {
          if (fullPath.startsWith(cleanSp)) {
            return { prefix: sp, subPath: fullPath.substring(cleanSp.length) };
          }
        } else {
          const apiSp = `api/${cleanSp}`;
          if (fullPath.startsWith(apiSp)) {
            return { prefix: sp, subPath: fullPath.substring(apiSp.length) };
          }
        }
      }

      // 3. Fallback: intelligent segment parsing
      if (fullPath.startsWith('api/')) {
        const afterApi = fullPath.substring('api/'.length);
        if (afterApi.includes('/')) {
          const slashIdx = afterApi.indexOf('/');
          const detectedPrefix = afterApi.substring(0, slashIdx + 1); // e.g. "banks/"
          const detectedSubPath = afterApi.substring(slashIdx + 1);    // e.g. "my-banks"
          saveCustomPrefix(detectedPrefix);
          return { prefix: detectedPrefix, subPath: detectedSubPath };
        } else {
          return { prefix: '', subPath: afterApi };
        }
      } else {
        // Root direct
        if (fullPath.includes('/')) {
          const slashIdx = fullPath.indexOf('/');
          const detectedPrefix = '/' + fullPath.substring(0, slashIdx + 1); // e.g. "/webhook/"
          const detectedSubPath = fullPath.substring(slashIdx + 1);
          saveCustomPrefix(detectedPrefix);
          return { prefix: detectedPrefix, subPath: detectedSubPath };
        } else {
          return { prefix: '/', subPath: fullPath };
        }
      }
    }

    function renderPrefixDropdownOptions(selectedVal) {
      const sel = document.getElementById('epPrefixSelect');
      if (!sel) return;

      const current = selectedVal !== undefined ? selectedVal : (sel.value || '');
      const savedList = getSavedCustomPrefixes();

      let html = `<option value="">(Default: /api/)</option>`;
      savedList.forEach(p => {
        let label;
        if (p.startsWith('/') && !p.startsWith('/api')) {
          label = `${p} (Root Direct: without /api/)`;
        } else {
          const cleanP = p.replace(/^\/?api\/?/, '');
          label = `${cleanP} (/api/${cleanP})`;
        }
        html += `<option value="${p}">${label}</option>`;
      });
      html += `<option value="/">/ (Root Direct: without /api/)</option>`;
      html += `<option value="__custom__">Custom prefix...</option>`;

      sel.innerHTML = html;
      sel.value = current;

      if (current && sel.value !== current) {
        sel.value = '__custom__';
        const customInput = document.getElementById('epCustomPrefixInput');
        if (customInput) customInput.value = current;
      }

      onCustomEpPrefixChange(sel.value);
    }

    function onCustomEpPrefixChange(val) {
      const wrapper = document.getElementById('epCustomPrefixWrapper');
      const customInput = document.getElementById('epCustomPrefixInput');
      const delBtn = document.getElementById('deletePrefixBtn');
      const pathEl = document.getElementById('epPathInput');

      if (val === 'custom' || val === '__custom__') {
        if (wrapper) wrapper.classList.remove('hidden');
        if (customInput) customInput.focus();
      } else {
        if (wrapper) wrapper.classList.add('hidden');
      }

      // If switching prefix and pathEl starts with the previous prefix, strip it!
      if (pathEl && lastSelectedPrefix && lastSelectedPrefix !== '/' && lastSelectedPrefix !== '__custom__') {
        const cleanOld = lastSelectedPrefix.replace(/^\/?api\/?/, '').replace(/^\/+/, '');
        if (cleanOld && pathEl.value.startsWith(cleanOld)) {
          pathEl.value = pathEl.value.substring(cleanOld.length).replace(/^\/+/, '');
        }
      }

      const newSelected = getSelectedPrefix();
      if (newSelected !== '__custom__') {
        lastSelectedPrefix = newSelected;
      }

      if (delBtn) {
        const saved = getSavedCustomPrefixes();
        delBtn.classList.toggle('hidden', !saved.includes(val));
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

      const prefix = getSelectedPrefix(); // e.g. "", saved custom prefix, "/", or custom

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

      // Don't duplicate if user already typed the sub-prefix (e.g. "banks/" when prefix is "banks/")
      if (cleanSub && rawSlug.startsWith(cleanSub)) {
        rawSlug = rawSlug.substring(cleanSub.length);
      }

      // If user had an old prefix that is being replaced, don't keep the old prefix in rawSlug
      if (lastSelectedPrefix && lastSelectedPrefix !== prefix) {
        const cleanOld = lastSelectedPrefix.replace(/^\/?api\/?/, '').replace(/^\/+/, '');
        if (cleanOld && rawSlug.startsWith(cleanOld)) {
          rawSlug = rawSlug.substring(cleanOld.length);
        }
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
        } else if (prefix.startsWith('/') && !prefix.startsWith('/api')) {
          fullPath = `/${prefix.replace(/^\/+/, '')}endpoint`.replace(/\/+/g, '/');
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
      const overrideBanner = document.getElementById('epOverrideNoticeBanner');
      if (overrideBanner) {
        overrideBanner.classList.toggle('hidden', !isOverride);
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
        pathEl.value = `api/${col}`;
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
        pathEl.value = `api/${col}/{id}`;
        txEl.checked = false;
        sqlEl.value = `SELECT * FROM \`${col}\` WHERE \`id\` = :id LIMIT 1;`;
        if (paramsEl) paramsEl.value = '{\n  "id": "1"\n}';
      } else if (preset === 'create_record') {
        nameEl.value = `Override Create ${col}`;
        methodEl.value = 'POST';
        pathEl.value = `api/${col}`;
        txEl.checked = true;
        sqlEl.value = `SELECT * FROM \`${col}\` ORDER BY \`id\` DESC LIMIT 1;`;
      } else if (preset === 'update_record') {
        nameEl.value = `Override Update ${col}`;
        methodEl.value = 'PATCH';
        pathEl.value = `api/${col}/{id}`;
        txEl.checked = true;
        sqlEl.value = `UPDATE \`${col}\` SET \`updated\` = CURRENT_TIMESTAMP WHERE \`id\` = :id;\nSELECT * FROM \`${col}\` WHERE \`id\` = :id LIMIT 1;`;
      } else if (preset === 'delete_record') {
        nameEl.value = `Override Delete ${col}`;
        methodEl.value = 'DELETE';
        pathEl.value = `api/${col}/{id}`;
        txEl.checked = true;
        sqlEl.value = `DELETE FROM \`${col}\` WHERE \`id\` = :id;`;
      }

      document.getElementById('epOverridePresetSelect').value = '';
      updateCustomEpUrlPreview();
    }

    async function loadCustomEndpoints(selectIdOrPath) {
      renderPrefixDropdownOptions();
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
        const fullDisplayPath = resolveEndpointFullPath(ep.path);
        const isOverride = isOverrideEndpointPath(ep.path);
        const method = ep.method || 'GET';
        return `
          <div class="group relative flex items-center rounded-2xl transition border ${isActive ? 'bg-slate-200/90 text-slate-900 border-slate-300 shadow-xs font-bold' : 'bg-white hover:bg-slate-100 text-slate-700 border-slate-200/60 font-semibold'}">
            <button type="button" onclick="selectCustomEndpoint('${ep.id}')" class="flex-1 flex items-center justify-between p-2.5 text-xs min-w-0">
              <div class="flex items-center space-x-2.5 truncate">
                <span class="w-8 h-8 rounded-xl flex items-center justify-center shrink-0 ${isActive ? 'bg-white text-slate-800 shadow-2xs' : 'bg-slate-100 text-slate-600'}">
                  <i class="fa-solid fa-code text-[11px]"></i>
                </span>
                <div class="text-left truncate">
                  <div class="truncate font-mono">${ep.name || ep.path}</div>
                  <div class="text-[10px] ${isActive ? 'text-slate-500 font-medium' : 'text-slate-400'}">${fullDisplayPath}</div>
                </div>
              </div>
              <div class="flex items-center space-x-1 shrink-0 ml-1">
                ${isOverride ? `<span class="px-1.5 py-0.5 rounded text-[8px] font-mono font-extrabold tracking-wider ${isActive ? 'bg-amber-400 text-amber-950 shadow-2xs' : 'bg-amber-100 text-amber-900 border border-amber-300'}" title="Overrides Built-in Route">OVERRIDE</span>` : ''}
                <span class="px-2 py-0.5 rounded-lg text-[9px] font-mono font-extrabold ${isActive ? 'bg-white text-slate-800 shadow-2xs border border-slate-200' : 'bg-slate-100 text-slate-700'}">${method}</span>
              </div>
            </button>
            <button type="button" onclick="selectCustomEndpoint('${ep.id}')" class="p-2 mr-1 rounded-lg ${isActive ? 'text-slate-600 hover:text-slate-900 hover:bg-slate-300/60' : 'text-slate-400 hover:bg-slate-100 hover:text-slate-700'} transition" title="Edit Endpoint">
              <i class="fa-solid fa-sliders text-[11px]"></i>
            </button>
          </div>
        `;
      }).join('');
    }

    // ================= TRANSACTION AUTO-DETECTION =================
    let txUserManuallySet = false;

    function countSqlStatements(sql) {
      if (!sql) return 0;
      const noComments = sql
        .replace(/--.*$/gm, '')
        .replace(/\/\*[\s\S]*?\*\//g, '');

      let count = 0;
      let current = '';
      let inSingle = false;
      let inDouble = false;
      let inBacktick = false;

      for (let i = 0; i < noComments.length; i++) {
        const c = noComments[i];
        if (c === "'" && !inDouble && !inBacktick) {
          if (inSingle && noComments[i + 1] === "'") {
            current += "''";
            i++;
            continue;
          }
          inSingle = !inSingle;
        } else if (c === '"' && !inSingle && !inBacktick) {
          inDouble = !inDouble;
        } else if (c === '`' && !inSingle && !inDouble) {
          inBacktick = !inBacktick;
        } else if (c === ';' && !inSingle && !inDouble && !inBacktick) {
          if (current.trim()) {
            count++;
          }
          current = '';
          continue;
        }
        current += c;
      }
      if (current.trim()) {
        count++;
      }
      return count;
    }

    function onUserToggleTransaction(checked) {
      txUserManuallySet = true;
      const badgeEl = document.getElementById('epTransactionBadge');
      if (badgeEl) {
        badgeEl.textContent = 'Manual';
        badgeEl.className = 'text-[9px] font-mono font-bold px-1.5 py-0.5 rounded bg-slate-100 text-slate-600';
        badgeEl.classList.remove('hidden');
      }
    }

    function autoDetectTransactionCheckbox() {
      const txEl = document.getElementById('epTransactionInput');
      const sqlEl = document.getElementById('epSqlInput');
      if (!txEl || !sqlEl) return;

      const count = countSqlStatements(sqlEl.value);
      const badgeEl = document.getElementById('epTransactionBadge');

      if (!txUserManuallySet) {
        if (count > 1) {
          txEl.checked = true;
          if (badgeEl) {
            badgeEl.textContent = 'Auto: multi-query';
            badgeEl.className = 'text-[9px] font-mono font-bold px-1.5 py-0.5 rounded bg-emerald-100 text-emerald-800';
            badgeEl.classList.remove('hidden');
          }
        } else {
          txEl.checked = false;
          if (badgeEl) {
            badgeEl.classList.add('hidden');
          }
        }
      }
    }

    function startNewCustomEndpoint() {
      currentCustomEpId = null;
      lastSelectedPrefix = '';
      originalEndpointPath = '';
      renderCustomEndpointsList();
      const alertEl = document.getElementById('customEpAlert');
      if (alertEl) alertEl.classList.add('hidden');

      document.getElementById('customEpFormTitle').textContent = 'Create Custom API Endpoint';
      document.getElementById('saveCustomEpBtnText').textContent = 'Create Endpoint';
      document.getElementById('deleteCustomEpBtn').classList.add('hidden');

      document.getElementById('epNameInput').value = '';
      document.getElementById('epMethodInput').value = 'GET';
      renderPrefixDropdownOptions('');
      const customPrefixEl = document.getElementById('epCustomPrefixInput');
      if (customPrefixEl) {
        customPrefixEl.value = '';
      }
      document.getElementById('epPathInput').value = '';
      document.getElementById('epAuthRuleInput').value = 'public';
      updateRoleDropdownUI('epAuthRuleInput');
      
      txUserManuallySet = false;
      document.getElementById('epTransactionInput').checked = false;
      const badgeEl = document.getElementById('epTransactionBadge');
      if (badgeEl) badgeEl.classList.add('hidden');

      document.getElementById('epSqlInput').value = `SELECT * FROM \`${currentCollection || 'users'}\` LIMIT :limit;`;
      document.getElementById('epDescInput').value = '';
      document.getElementById('epTestParamsInput').value = '{\n  "limit": 10\n}';
      qbJoins = [];
      qbExtraTables = [];
      renderQbJoins();
      renderQbExtraTables();
      updateCustomEpUrlPreview();
      autoDetectTransactionCheckbox();
    }

    function selectCustomEndpoint(id) {
      const ep = customEndpoints.find(e => e.id === id);
      if (!ep) return;
      currentCustomEpId = ep.id;
      originalEndpointPath = ep.path || '';
      renderCustomEndpointsList();
      const alertEl = document.getElementById('customEpAlert');
      if (alertEl) alertEl.classList.add('hidden');

      document.getElementById('customEpFormTitle').textContent = `Edit Endpoint: ${ep.name || ep.path}`;
      document.getElementById('saveCustomEpBtnText').textContent = 'Save Changes';
      document.getElementById('deleteCustomEpBtn').classList.remove('hidden');

      document.getElementById('epNameInput').value = ep.name || '';
      document.getElementById('epMethodInput').value = ep.method || 'GET';

      // Parse path to automatically separate prefix vs slug
      const parsed = parseEndpointPathAndPrefix(ep);
      lastSelectedPrefix = parsed.prefix;

      renderPrefixDropdownOptions(parsed.prefix);
      const customPrefixEl = document.getElementById('epCustomPrefixInput');
      if (customPrefixEl) {
        if (parsed.prefix && !['', '/'].includes(parsed.prefix)) {
          customPrefixEl.value = parsed.prefix;
        } else {
          customPrefixEl.value = '';
        }
      }

      const pathEl = document.getElementById('epPathInput');
      if (pathEl) {
        pathEl.value = parsed.subPath;
      }

      document.getElementById('epAuthRuleInput').value = ep.authRule || ep.auth_rule || 'public';
      updateRoleDropdownUI('epAuthRuleInput');
      
      txUserManuallySet = true;
      document.getElementById('epTransactionInput').checked = Boolean(ep.transaction);
      const badgeEl = document.getElementById('epTransactionBadge');
      if (badgeEl) badgeEl.classList.add('hidden');

      document.getElementById('epSqlInput').value = ep.sql || '';
      document.getElementById('epDescInput').value = ep.description || '';
      qbJoins = [];
      qbExtraTables = [];
      renderQbJoins();
      renderQbExtraTables();
      updateCustomEpUrlPreview();
    }

    // ================= VISUAL QUERY BUILDER =================
    let qbEditorMode = 'code'; // 'code' | 'visual'
    let qbCurrentAction = 'SELECT'; // 'SELECT' | 'INSERT' | 'UPDATE' | 'DELETE' | 'COUNT'
    let qbSelectedTable = '';
    let qbSelectedColumns = new Set();
    let qbFilterConditions = []; // Array of { logic: 'AND', col: '', op: '=', val: ':val' }
    let qbJoins = []; // Array of { id, type: 'LEFT JOIN'|'INNER JOIN'|'RIGHT JOIN', table, mainCol, joinCol, selectedCols: Set() }
    let qbExtraTables = []; // Array of { id, table, selectedCols: Set(), whereCol, whereVal }
    let qbCurrentResultTab = 'json'; // 'json' | 'table'
    let qbLastResultData = null;

    function qbEscapeAttr(str) {
      if (str === null || str === undefined) return '';
      return String(str).replace(/"/g, '&quot;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
    }

    function switchEndpointEditorMode(mode) {
      qbEditorMode = mode;
      const codeBtn = document.getElementById('epModeCodeBtn');
      const visualBtn = document.getElementById('epModeVisualBtn');
      const codeIcon = document.getElementById('epModeCodeIcon');
      const visualIcon = document.getElementById('epModeVisualIcon');
      const codePane = document.getElementById('epSqlEditorPane');
      const visualPane = document.getElementById('epVisualBuilderPane');

      const baseClasses = "px-4 py-1.5 rounded-lg text-xs transition flex items-center space-x-1.5 cursor-pointer";

      if (mode === 'visual') {
        if (codeBtn) {
          codeBtn.className = `${baseClasses} mode-btn-inactive text-slate-500 hover:text-slate-800 hover:bg-white/60 font-semibold`;
        }
        if (codeIcon) {
          codeIcon.className = "fa-solid fa-code text-slate-400";
        }
        if (visualBtn) {
          visualBtn.className = `${baseClasses} mode-btn-active bg-emerald-600 text-white font-extrabold shadow-sm`;
        }
        if (visualIcon) {
          visualIcon.className = "fa-solid fa-wand-magic-sparkles text-white";
        }
        if (codePane) codePane.classList.add('hidden');
        if (visualPane) visualPane.classList.remove('hidden');

        initQueryBuilder();
      } else {
        if (codeBtn) {
          codeBtn.className = `${baseClasses} mode-btn-active bg-emerald-600 text-white font-extrabold shadow-sm`;
        }
        if (codeIcon) {
          codeIcon.className = "fa-solid fa-code text-white";
        }
        if (visualBtn) {
          visualBtn.className = `${baseClasses} mode-btn-inactive text-slate-500 hover:text-slate-800 hover:bg-white/60 font-semibold`;
        }
        if (visualIcon) {
          visualIcon.className = "fa-solid fa-wand-magic-sparkles text-slate-400";
        }
        if (codePane) codePane.classList.remove('hidden');
        if (visualPane) visualPane.classList.add('hidden');
      }
    }

    function initQueryBuilder() {
      populateQbTables();
      if (!qbSelectedTable && collections.length > 0) {
        qbSelectedTable = currentCollection || collections[0].name;
      }
      const sel = document.getElementById('qbTableSelect');
      if (sel && qbSelectedTable) {
        sel.value = qbSelectedTable;
        onQbTableChange(qbSelectedTable, false);
      }
      renderQbJoins();
      renderQbExtraTables();
      buildSqlFromQb(true);
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

      // Reset joins and extra tables on primary table switch
      qbJoins = [];
      renderQbJoins();
      qbExtraTables = [];
      renderQbExtraTables();

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
      const joinSec = document.getElementById('qbJoinSection');
      const multiSec = document.getElementById('qbMultiTableSection');
      const whereSec = document.getElementById('qbWhereSection');
      const multiLabel = document.getElementById('qbMultiTableLabel');

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

      if (joinSec) {
        if (action === 'SELECT' || action === 'COUNT') {
          joinSec.classList.remove('hidden');
        } else {
          joinSec.classList.add('hidden');
        }
      }

      if (multiSec) {
        if (action === 'INSERT' || action === 'UPDATE' || action === 'DELETE') {
          multiSec.classList.remove('hidden');
          if (multiLabel) {
            if (action === 'INSERT') multiLabel.textContent = 'Multi-Table Operations (Insert into multiple tables atomically)';
            else if (action === 'UPDATE') multiLabel.textContent = 'Multi-Table Operations (Update multiple tables atomically)';
            else if (action === 'DELETE') multiLabel.textContent = 'Multi-Table Operations (Cascade delete related tables atomically)';
          }
        } else {
          multiSec.classList.add('hidden');
        }
      }

      if (whereSec) {
        if (action === 'INSERT') {
          whereSec.classList.add('hidden');
        } else {
          whereSec.classList.remove('hidden');
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

      renderQbExtraTables();
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

    function applyQbOwnCreateFilter() {
      const table = qbSelectedTable || currentCollection || 'banks';
      const fields = getTableFieldsList(table);

      let ownCol = 'created_by';
      if (fields.some(f => f.name === 'created_by')) {
        ownCol = 'created_by';
      } else if (fields.some(f => f.name === 'user_id')) {
        ownCol = 'user_id';
      } else if (fields.some(f => f.name === 'author_id')) {
        ownCol = 'author_id';
      } else if (fields.some(f => f.name === 'owner_id')) {
        ownCol = 'owner_id';
      }

      const existing = qbFilterConditions.find(c => c.col === ownCol && c.val === '@request.auth.id');
      if (!existing) {
        qbFilterConditions.push({
          logic: qbFilterConditions.length === 0 ? 'WHERE' : 'AND',
          col: ownCol,
          op: '=',
          val: '@request.auth.id'
        });
      }

      const authRuleEl = document.getElementById('epAuthRuleInput');
      if (authRuleEl && (!authRuleEl.value || authRuleEl.value === 'public')) {
        authRuleEl.value = "@request.auth.id != ''";
        updateRoleDropdownUI('epAuthRuleInput');
      }

      renderQbFilters();
      buildSqlFromQb();
      if (typeof showToast === 'function') {
        showToast(`Added Own Create filter: WHERE \`${ownCol}\` = @request.auth.id`);
      }
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

    // ================= TABLE JOINS (SELECT) =================
    function addQbJoinRow() {
      const otherCols = collections.filter(c => c.name !== qbSelectedTable);
      if (otherCols.length === 0) {
        showToast('No other table available to JOIN', 'warning');
        return;
      }
      const candTable = otherCols[0].name;
      const mainFields = getTableFieldsList(qbSelectedTable);
      const candFields = getTableFieldsList(candTable);

      const guessMain = mainFields.find(f => 
        f.name === `${candTable}_id` || 
        f.name === `${candTable}Id` ||
        (candTable === 'users' && (f.name === 'user_id' || f.name === 'author_id' || f.name === 'customer_id'))
      )?.name || 'id';

      const guessJoin = candFields.find(f => f.name === 'id')?.name || (candFields[0]?.name || 'id');

      const selectedCols = new Set();
      candFields.filter(f => f.name !== 'id' && f.name !== 'created_at' && f.name !== 'updated_at')
        .slice(0, 2)
        .forEach(f => selectedCols.add(f.name));

      qbJoins.push({
        id: 'join_' + Date.now() + '_' + Math.random().toString(36).substring(2, 6),
        type: 'LEFT JOIN',
        table: candTable,
        mainCol: guessMain,
        joinCol: guessJoin,
        selectedCols
      });

      renderQbJoins();
      buildSqlFromQb();
    }

    function removeQbJoinRow(id) {
      qbJoins = qbJoins.filter(j => j.id !== id);
      renderQbJoins();
      buildSqlFromQb();
    }

    function onQbJoinChange(id, key, val) {
      const j = qbJoins.find(x => x.id === id);
      if (!j) return;
      j[key] = val;
      if (key === 'table') {
        const fields = getTableFieldsList(val);
        j.selectedCols = new Set();
        fields.filter(f => f.name !== 'id' && f.name !== 'created_at' && f.name !== 'updated_at')
          .slice(0, 2)
          .forEach(f => j.selectedCols.add(f.name));
        j.joinCol = fields.find(f => f.name === 'id')?.name || fields[0]?.name || 'id';
      }
      renderQbJoins();
      buildSqlFromQb();
    }

    function toggleQbJoinCol(id, colName) {
      const j = qbJoins.find(x => x.id === id);
      if (!j) return;
      if (j.selectedCols.has(colName)) {
        j.selectedCols.delete(colName);
      } else {
        j.selectedCols.add(colName);
      }
      renderQbJoins();
      buildSqlFromQb();
    }

    function toggleQbJoinAllCols(id, selectAll) {
      const j = qbJoins.find(x => x.id === id);
      if (!j) return;
      j.selectedCols.clear();
      if (selectAll) {
        const fields = getTableFieldsList(j.table);
        fields.forEach(f => j.selectedCols.add(f.name));
      }
      renderQbJoins();
      buildSqlFromQb();
    }

    function renderQbJoins() {
      const container = document.getElementById('qbJoinsContainer');
      if (!container) return;

      if (qbJoins.length === 0) {
        container.innerHTML = '<div id="qbEmptyJoinsNotice" class="text-xs text-slate-400 italic py-1">No JOIN configured. Query only fetches from the primary table.</div>';
        return;
      }

      const mainTable = qbSelectedTable || 'users';
      const mainFields = getTableFieldsList(mainTable);

      container.innerHTML = qbJoins.map((j, idx) => {
        const joinFields = getTableFieldsList(j.table);
        const joinColOptions = joinFields.map(f => `<option value="${f.name}" ${f.name === j.joinCol ? 'selected' : ''}>\`${f.name}\`</option>`).join('');
        const mainColOptions = mainFields.map(f => `<option value="${f.name}" ${f.name === j.mainCol ? 'selected' : ''}>\`${f.name}\`</option>`).join('');

        return `
          <div class="bg-indigo-50/40 border border-indigo-200/80 rounded-xl p-3 space-y-2.5 text-xs shadow-2xs">
            <div class="flex items-center justify-between flex-wrap gap-2">
              <div class="flex items-center space-x-2 flex-wrap">
                <span class="px-2 py-0.5 rounded bg-indigo-600 text-white font-mono font-bold text-[10px]">
                  JOIN #${idx + 1}
                </span>
                <select onchange="onQbJoinChange('${j.id}', 'type', this.value)" class="bg-white border border-indigo-300 rounded-lg px-2 py-1 font-bold text-indigo-700 text-xs font-mono">
                  <option value="LEFT JOIN" ${j.type === 'LEFT JOIN' ? 'selected' : ''}>LEFT JOIN</option>
                  <option value="INNER JOIN" ${j.type === 'INNER JOIN' ? 'selected' : ''}>INNER JOIN</option>
                  <option value="RIGHT JOIN" ${j.type === 'RIGHT JOIN' ? 'selected' : ''}>RIGHT JOIN</option>
                </select>
                <span class="text-slate-500 font-bold">table</span>
                <select onchange="onQbJoinChange('${j.id}', 'table', this.value)" class="bg-white border border-indigo-300 rounded-lg px-2.5 py-1 font-mono font-bold text-slate-800 text-xs">
                  ${collections.map(c => `<option value="${c.name}" ${c.name === j.table ? 'selected' : ''}>${c.name}</option>`).join('')}
                </select>
              </div>

              <button type="button" onclick="removeQbJoinRow('${j.id}')" class="p-1 text-slate-400 hover:text-rose-600 transition rounded-lg hover:bg-rose-50 cursor-pointer" title="Remove JOIN">
                <i class="fa-solid fa-trash-can text-xs"></i>
              </button>
            </div>

            <!-- ON Condition -->
            <div class="flex items-center flex-wrap gap-2 bg-white/90 p-2 rounded-lg border border-indigo-100 text-xs font-mono">
              <span class="font-bold text-indigo-700">ON</span>
              <span class="text-slate-500 font-semibold">\`${mainTable}\`.</span>
              <select onchange="onQbJoinChange('${j.id}', 'mainCol', this.value)" class="bg-slate-50 border border-slate-300 rounded px-2 py-1 text-xs">
                ${mainColOptions}
              </select>
              <span class="font-bold text-slate-600">=</span>
              <span class="text-slate-500 font-semibold">\`${j.table}\`.</span>
              <select onchange="onQbJoinChange('${j.id}', 'joinCol', this.value)" class="bg-slate-50 border border-slate-300 rounded px-2 py-1 text-xs">
                ${joinColOptions}
              </select>
            </div>

            <!-- Column chips from joined table -->
            <div>
              <div class="flex items-center justify-between mb-1.5 text-[11px]">
                <span class="font-bold text-slate-700">Columns to include from \`${j.table}\`:</span>
                <div class="space-x-1.5">
                  <button type="button" onclick="toggleQbJoinAllCols('${j.id}', true)" class="text-[10px] text-indigo-600 font-bold hover:underline cursor-pointer">Select All</button>
                  <span class="text-slate-300">|</span>
                  <button type="button" onclick="toggleQbJoinAllCols('${j.id}', false)" class="text-[10px] text-slate-500 hover:underline cursor-pointer">Clear</button>
                </div>
              </div>
              <div class="flex flex-wrap gap-1 max-h-24 overflow-y-auto p-1 bg-white/60 rounded-lg border border-indigo-50">
                ${joinFields.map(f => {
                  const isSel = j.selectedCols.has(f.name);
                  return `
                    <button type="button" onclick="toggleQbJoinCol('${j.id}', '${f.name}')" class="px-2 py-0.5 rounded text-[11px] font-mono transition border cursor-pointer ${
                      isSel ? 'bg-indigo-600 text-white border-indigo-700 font-bold shadow-2xs' : 'bg-white text-slate-600 border-slate-200 hover:border-slate-300'
                    }">
                      ${isSel ? '<i class="fa-solid fa-check text-[9px] mr-1"></i>' : ''}${f.name}
                    </button>
                  `;
                }).join('')}
              </div>
            </div>
          </div>
        `;
      }).join('');
    }

    // ================= MULTI-TABLE OPERATIONS (INSERT, UPDATE, DELETE) =================
    function addQbExtraTableRow() {
      const otherCols = collections.filter(c => c.name !== qbSelectedTable);
      const candTable = otherCols.length > 0 ? otherCols[0].name : (collections[0]?.name || 'items');
      const candFields = getTableFieldsList(candTable);

      const selectedCols = new Set();
      candFields.filter(f => f.name !== 'id' && f.name !== 'created_at' && f.name !== 'updated_at')
        .slice(0, 3)
        .forEach(f => selectedCols.add(f.name));

      const guessWhere = candFields.find(f => 
        f.name === `${qbSelectedTable}_id` || 
        f.name === `${qbSelectedTable}Id` ||
        f.name.includes(`${qbSelectedTable}`)
      )?.name || 'id';

      const colBindings = {};
      const primaryTable = qbSelectedTable || 'orders';
      const defaultFk = candFields.find(f => 
        f.name === `${primaryTable}_id` || 
        f.name === `${primaryTable.replace(/s$/, '')}_id` ||
        (primaryTable === 'orders' && f.name === 'order_id') ||
        (primaryTable === 'users' && f.name === 'user_id')
      )?.name;

      if (defaultFk) {
        colBindings[defaultFk] = ':last_insert_id';
        selectedCols.add(defaultFk);
      }

      qbExtraTables.push({
        id: 'extra_' + Date.now() + '_' + Math.random().toString(36).substring(2, 6),
        table: candTable,
        selectedCols,
        colBindings,
        whereCol: guessWhere,
        whereVal: `:${guessWhere}`
      });

      renderQbExtraTables();
      buildSqlFromQb();
      showToast(`Table '${candTable}' added to multi-table transaction`);
    }

    function removeQbExtraTableRow(id) {
      qbExtraTables = qbExtraTables.filter(x => x.id !== id);
      renderQbExtraTables();
      buildSqlFromQb();
    }

    function onQbExtraTableChange(id, key, val) {
      const extra = qbExtraTables.find(x => x.id === id);
      if (!extra) return;
      extra[key] = val;
      if (key === 'table') {
        const fields = getTableFieldsList(val);
        extra.selectedCols = new Set();
        fields.filter(f => f.name !== 'id' && f.name !== 'created_at' && f.name !== 'updated_at')
          .slice(0, 3)
          .forEach(f => extra.selectedCols.add(f.name));
        extra.whereCol = fields.find(f => f.name === 'id')?.name || fields[0]?.name || 'id';
        extra.whereVal = `:${extra.whereCol}`;

        // Reset and guess foreign key
        extra.colBindings = {};
        const primaryTable = qbSelectedTable || 'orders';
        const defaultFk = fields.find(f => 
          f.name === `${primaryTable}_id` || 
          f.name === `${primaryTable.replace(/s$/, '')}_id` ||
          (primaryTable === 'orders' && f.name === 'order_id') ||
          (primaryTable === 'users' && f.name === 'user_id')
        )?.name;
        if (defaultFk) {
          extra.colBindings[defaultFk] = ':last_insert_id';
          extra.selectedCols.add(defaultFk);
        }
      }
      renderQbExtraTables();
      buildSqlFromQb();
    }

    function setQbExtraForeignKey(extraId, colName) {
      const extra = qbExtraTables.find(x => x.id === extraId);
      if (!extra) return;
      if (!extra.colBindings) extra.colBindings = {};
      Object.keys(extra.colBindings).forEach(k => {
        if (extra.colBindings[k] === ':last_insert_id') delete extra.colBindings[k];
      });
      if (colName) {
        extra.colBindings[colName] = ':last_insert_id';
        extra.selectedCols.add(colName);
      }
      renderQbExtraTables();
      buildSqlFromQb();
    }

    function toggleQbExtraTableCol(id, colName) {
      const extra = qbExtraTables.find(x => x.id === id);
      if (!extra) return;
      if (extra.selectedCols.has(colName)) {
        extra.selectedCols.delete(colName);
      } else {
        extra.selectedCols.add(colName);
      }
      renderQbExtraTables();
      buildSqlFromQb();
    }

    function toggleQbExtraTableAllCols(id, selectAll) {
      const extra = qbExtraTables.find(x => x.id === id);
      if (!extra) return;
      extra.selectedCols.clear();
      if (selectAll) {
        const fields = getTableFieldsList(extra.table);
        fields.filter(f => f.name !== 'id' && f.name !== 'created_at' && f.name !== 'updated_at')
          .forEach(f => extra.selectedCols.add(f.name));
      }
      renderQbExtraTables();
      buildSqlFromQb();
    }

    function renderQbExtraTables() {
      const container = document.getElementById('qbExtraTablesContainer');
      if (!container) return;

      if (qbExtraTables.length === 0) {
        container.innerHTML = '<div id="qbEmptyExtraTablesNotice" class="text-xs text-slate-400 italic py-1">Single table operation. Click "+ Add Another Table" to chain mutations in this transaction.</div>';
        return;
      }

      container.innerHTML = qbExtraTables.map((extra, idx) => {
        const extraFields = getTableFieldsList(extra.table);
        const whereColOptions = extraFields.map(f => `<option value="${f.name}" ${f.name === extra.whereCol ? 'selected' : ''}>\`${f.name}\`</option>`).join('');

        return `
          <div class="bg-teal-50/40 border border-teal-200/80 rounded-xl p-3 space-y-2.5 text-xs shadow-2xs">
            <div class="flex items-center justify-between flex-wrap gap-2">
              <div class="flex items-center space-x-2 flex-wrap">
                <span class="px-2 py-0.5 rounded bg-teal-600 text-white font-mono font-bold text-[10px] uppercase">
                  Additional Table #${idx + 1} (${qbCurrentAction})
                </span>
                <span class="text-slate-500 font-bold">table</span>
                <select onchange="onQbExtraTableChange('${extra.id}', 'table', this.value)" class="bg-white border border-teal-300 rounded-lg px-2.5 py-1 font-mono font-bold text-slate-800 text-xs">
                  ${collections.map(c => `<option value="${c.name}" ${c.name === extra.table ? 'selected' : ''}>${c.name}</option>`).join('')}
                </select>
              </div>

              <button type="button" onclick="removeQbExtraTableRow('${extra.id}')" class="p-1 text-slate-400 hover:text-rose-600 transition rounded-lg hover:bg-rose-50 cursor-pointer" title="Remove Table">
                <i class="fa-solid fa-trash-can text-xs"></i>
              </button>
            </div>

            <!-- Columns for INSERT or UPDATE -->
            ${(qbCurrentAction === 'INSERT' || qbCurrentAction === 'UPDATE') ? `
              <div>
                <div class="flex items-center justify-between mb-1.5 text-[11px]">
                  <span class="font-bold text-slate-700">${qbCurrentAction === 'INSERT' ? 'Columns to INSERT (values):' : 'Columns to UPDATE (SET):'}</span>
                  <div class="space-x-1.5">
                    <button type="button" onclick="toggleQbExtraTableAllCols('${extra.id}', true)" class="text-[10px] text-teal-700 font-bold hover:underline cursor-pointer">Select All</button>
                    <span class="text-slate-300">|</span>
                    <button type="button" onclick="toggleQbExtraTableAllCols('${extra.id}', false)" class="text-[10px] text-slate-500 hover:underline cursor-pointer">Clear</button>
                  </div>
                </div>
                <div class="flex flex-wrap gap-1 max-h-24 overflow-y-auto p-1 bg-white/60 rounded-lg border border-teal-50">
                  ${extraFields.filter(f => f.name !== 'id' && f.name !== 'created_at' && f.name !== 'updated_at').map(f => {
                    const isSel = extra.selectedCols.has(f.name);
                    const isBoundToLastId = extra.colBindings && extra.colBindings[f.name] === ':last_insert_id';
                    return `
                      <button type="button" onclick="toggleQbExtraTableCol('${extra.id}', '${f.name}')" class="px-2 py-0.5 rounded text-[11px] font-mono transition border cursor-pointer ${
                        isBoundToLastId
                          ? 'bg-amber-500 text-white border-amber-600 font-extrabold shadow-2xs'
                          : (isSel ? 'bg-teal-600 text-white border-teal-700 font-bold shadow-2xs' : 'bg-white text-slate-600 border-slate-200 hover:border-slate-300')
                      }">
                        ${isBoundToLastId ? '<i class="fa-solid fa-link text-[9px] mr-1"></i>' : (isSel ? '<i class="fa-solid fa-check text-[9px] mr-1"></i>' : '')}${f.name}${isBoundToLastId ? ' (🔗 :last_insert_id)' : ''}
                      </button>
                    `;
                  }).join('')}
                </div>

                ${qbCurrentAction === 'INSERT' ? `
                  <!-- Auto-link :last_insert_id for Foreign Key -->
                  <div class="mt-2 bg-amber-50/80 border border-amber-200/90 rounded-lg p-2.5 flex items-center justify-between flex-wrap gap-2 text-xs">
                    <div class="flex items-center space-x-1.5 text-amber-900 font-semibold flex-wrap">
                      <i class="fa-solid fa-link text-amber-600"></i>
                      <span>Auto-link parent ID:</span>
                      <select onchange="setQbExtraForeignKey('${extra.id}', this.value)" class="bg-white border border-amber-300 rounded-lg px-2 py-0.5 text-xs font-mono font-bold text-amber-900">
                        <option value="">(None - manual param)</option>
                        ${extraFields.filter(f => f.name !== 'id' && f.name !== 'created_at' && f.name !== 'updated_at').map(f => {
                          const isBound = extra.colBindings && extra.colBindings[f.name] === ':last_insert_id';
                          return `<option value="${f.name}" ${isBound ? 'selected' : ''}>\`${f.name}\` → :last_insert_id</option>`;
                        }).join('')}
                      </select>
                    </div>
                    <span class="text-[10px] text-amber-700">Captures ID from statement 1 automatically</span>
                  </div>
                ` : ''}
              </div>
            ` : ''}

            <!-- WHERE Condition for UPDATE or DELETE -->
            ${(qbCurrentAction === 'UPDATE' || qbCurrentAction === 'DELETE') ? `
              <div class="flex items-center flex-wrap gap-2 bg-white/90 p-2 rounded-lg border border-teal-100 text-xs font-mono">
                <span class="font-bold text-teal-700">WHERE</span>
                <span class="text-slate-500 font-semibold">\`${extra.table}\`.</span>
                <select onchange="onQbExtraTableChange('${extra.id}', 'whereCol', this.value)" class="bg-slate-50 border border-slate-300 rounded px-2 py-1 text-xs">
                  ${whereColOptions}
                </select>
                <span class="font-bold text-slate-600">=</span>
                <input type="text" oninput="onQbExtraTableChange('${extra.id}', 'whereVal', this.value)" value="${qbEscapeAttr(extra.whereVal || `:${extra.whereCol || 'id'}`)}" placeholder=":${extra.whereCol || 'id'}" class="bg-slate-50 border border-slate-300 rounded px-2 py-1 text-xs flex-1 min-w-[120px]">
              </div>
            ` : ''}
          </div>
        `;
      }).join('');
    }

    function applyQbPlainText(message = 'hello') {
      const qbSql = document.getElementById('qbSqlOutput');
      const qbParams = document.getElementById('qbParamsOutput');
      if (qbSql) qbSql.value = `'${message}'`;
      if (qbParams) qbParams.value = '{}';

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

      if (typeof updateCustomEpUrlPreview === 'function') {
        updateCustomEpUrlPreview();
      }
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
          const colPrefix = qbJoins.length > 0 ? `\`${table}\`.` : '';
          if (c.op === 'IS NULL' || c.op === 'IS NOT NULL') {
            return `${prefix}${colPrefix}\`${c.col}\` ${c.op}`;
          } else if (c.op === 'LIKE') {
            const paramName = (c.val || `:${c.col}`).replace(/^:/, '');
            paramsObj[paramName] = "test";
            return `${prefix}${colPrefix}\`${c.col}\` LIKE '%' || :${paramName} || '%'`;
          } else {
            const val = c.val || `:${c.col}`;
            if (val.startsWith(':')) {
              const pName = val.replace(/^:/, '');
              paramsObj[pName] = pName === 'id' ? "1" : (pName.includes('limit') ? 10 : "active");
            }
            return `${prefix}${colPrefix}\`${c.col}\` ${c.op} ${val}`;
          }
        });
        whereClause = `\nWHERE ${parts.join('')}`;
      }

      if (qbCurrentAction === 'SELECT') {
        const hasJoins = qbJoins.length > 0;
        let colsList = '*';

        if (hasJoins) {
          const mainCols = (qbSelectedColumns.size > 0 && qbSelectedColumns.size < allFields.length)
            ? Array.from(qbSelectedColumns).map(c => `\`${table}\`.\`${c}\``).join(', ')
            : `\`${table}\`.*`;

          const joinCols = [];
          qbJoins.forEach(j => {
            j.selectedCols.forEach(c => {
              joinCols.push(`\`${j.table}\`.\`${c}\` AS \`${j.table}_${c}\``);
            });
          });

          colsList = [mainCols, ...joinCols].join(',\n  ');
        } else {
          if (qbSelectedColumns.size > 0 && qbSelectedColumns.size < allFields.length) {
            colsList = Array.from(qbSelectedColumns).map(c => `\`${c}\``).join(', ');
          }
        }

        const orderByCol = document.getElementById('qbOrderByCol')?.value;
        const orderDir = document.getElementById('qbOrderDir')?.value || 'DESC';
        let orderClause = '';
        if (orderByCol) {
          orderClause = hasJoins
            ? `\nORDER BY \`${table}\`.\`${orderByCol}\` ${orderDir}`
            : `\nORDER BY \`${orderByCol}\` ${orderDir}`;
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

        let joinClauses = '';
        if (hasJoins) {
          joinClauses = qbJoins.map(j => {
            return `\n${j.type} \`${j.table}\` ON \`${table}\`.\`${j.mainCol}\` = \`${j.table}\`.\`${j.joinCol}\``;
          }).join('');
        }

        sql = `SELECT\n  ${colsList}\nFROM \`${table}\`${joinClauses}${whereClause}${orderClause}${limitClause};`;

      } else if (qbCurrentAction === 'INSERT') {
        const hasExtra = qbExtraTables.length > 0;
        const targetCols = qbSelectedColumns.size > 0 
          ? Array.from(qbSelectedColumns).filter(c => c !== 'id' && c !== 'created_at' && c !== 'updated_at')
          : allFields.map(f => f.name).filter(c => c !== 'id' && c !== 'created_at' && c !== 'updated_at');

        const colNames = targetCols.map(c => `\`${c}\``).join(', ');
        const valNames = targetCols.map(c => `:${c}`).join(', ');

        targetCols.forEach(c => {
          const f = allFields.find(x => x.name === c);
          paramsObj[c] = f?.type === 'number' ? 100 : (f?.type === 'bool' ? true : `sample_${c}`);
        });

        if (!hasExtra) {
          sql = `INSERT INTO \`${table}\` (${colNames})\nVALUES (${valNames});`;
        } else {
          let stmts = [];
          stmts.push(`-- ================================================================\n-- 📦 Multi-Table Atomic Transaction: INSERT\n-- All operations below run inside a single database transaction.\n-- ================================================================\n\n-- 1. Insert into primary table: \`${table}\`\nINSERT INTO \`${table}\` (${colNames})\nVALUES (${valNames});`);

          qbExtraTables.forEach((extra, idx) => {
            const extraFields = getTableFieldsList(extra.table);
            const extraCols = extra.selectedCols.size > 0
              ? Array.from(extra.selectedCols)
              : extraFields.map(f => f.name).filter(c => c !== 'id' && c !== 'created_at' && c !== 'updated_at');

            const eColNames = extraCols.map(c => `\`${c}\``).join(', ');
            let usesLastId = false;
            const eValNames = extraCols.map(c => {
              const isForeignKeyToPrimary = 
                c === `${table}_id` || 
                c === `${table.replace(/s$/, '')}_id` || 
                (table === 'orders' && c === 'order_id') ||
                (table === 'users' && c === 'user_id');

              if (isForeignKeyToPrimary) {
                usesLastId = true;
                return ':last_insert_id';
              }

              const pName = targetCols.includes(c) ? `${extra.table}_${c}` : c;
              const f = extraFields.find(x => x.name === c);
              paramsObj[pName] = f?.type === 'number' ? 150 : (f?.type === 'bool' ? true : `sample_${pName}`);
              return `:${pName}`;
            }).join(', ');

            const note = usesLastId ? '\n-- `:last_insert_id` is automatically populated from the insert above.' : '';
            stmts.push(`\n-- ${idx + 2}. Insert into secondary table: \`${extra.table}\`${note}\nINSERT INTO \`${extra.table}\` (${eColNames})\nVALUES (${eValNames});`);
          });

          sql = stmts.join('\n');
        }

      } else if (qbCurrentAction === 'UPDATE') {
        const hasExtra = qbExtraTables.length > 0;
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

        if (!hasExtra) {
          sql = `UPDATE \`${table}\`\nSET ${setClauses}${effectiveWhere};`;
        } else {
          let stmts = [];
          stmts.push(`-- ================================================================\n-- ✏️ Multi-Table Atomic Transaction: UPDATE\n-- ================================================================\n\n-- 1. Update primary table: \`${table}\`\nUPDATE \`${table}\`\nSET ${setClauses}${effectiveWhere};`);

          qbExtraTables.forEach((extra, idx) => {
            const extraFields = getTableFieldsList(extra.table);
            const extraCols = extra.selectedCols.size > 0
              ? Array.from(extra.selectedCols)
              : extraFields.map(f => f.name).filter(c => c !== 'id' && c !== 'created_at' && c !== 'updated_at');

            const eSetClauses = extraCols.map(c => {
              const pName = targetCols.includes(c) ? `${extra.table}_${c}` : c;
              const f = extraFields.find(x => x.name === c);
              paramsObj[pName] = f?.type === 'number' ? 250 : (f?.type === 'bool' ? false : `updated_${pName}`);
              return `\`${c}\` = :${pName}`;
            }).join(', ');

            const wCol = extra.whereCol || 'id';
            const wVal = extra.whereVal || `:${wCol}`;
            if (wVal.startsWith(':')) {
              const pName = wVal.replace(/^:/, '');
              paramsObj[pName] = "1";
            }

            stmts.push(`\n-- ${idx + 2}. Update secondary table: \`${extra.table}\`\nUPDATE \`${extra.table}\`\nSET ${eSetClauses}\nWHERE \`${wCol}\` = ${wVal};`);
          });

          sql = stmts.join('\n');
        }

      } else if (qbCurrentAction === 'DELETE') {
        const hasExtra = qbExtraTables.length > 0;
        const effectiveWhere = whereClause ? whereClause : '\nWHERE `id` = :id';
        if (!whereClause) paramsObj['id'] = "1";

        if (!hasExtra) {
          sql = `DELETE FROM \`${table}\`${effectiveWhere};`;
        } else {
          let stmts = [];
          stmts.push(`-- ================================================================\n-- 🗑️ Multi-Table Atomic Transaction: DELETE\n-- Dependent child records are deleted before primary records.\n-- ================================================================`);

          qbExtraTables.forEach((extra, idx) => {
            const wCol = extra.whereCol || 'id';
            const wVal = extra.whereVal || `:${wCol}`;
            if (wVal.startsWith(':')) {
              const pName = wVal.replace(/^:/, '');
              paramsObj[pName] = "1";
            }
            stmts.push(`\n-- ${idx + 1}. Delete dependent records from: \`${extra.table}\`\nDELETE FROM \`${extra.table}\`\nWHERE \`${wCol}\` = ${wVal};`);
          });

          stmts.push(`\n-- ${qbExtraTables.length + 1}. Delete primary record from: \`${table}\`\nDELETE FROM \`${table}\`${effectiveWhere};`);

          sql = stmts.join('\n');
        }

      } else if (qbCurrentAction === 'COUNT') {
        const hasJoins = qbJoins.length > 0;
        let joinClauses = '';
        if (hasJoins) {
          joinClauses = qbJoins.map(j => {
            return `\n${j.type} \`${j.table}\` ON \`${table}\`.\`${j.mainCol}\` = \`${j.table}\`.\`${j.joinCol}\``;
          }).join('');
        }
        sql = `SELECT COUNT(*) AS total_count\nFROM \`${table}\`${joinClauses}${whereClause} LIMIT 1;`;
      }

      const qbSqlEl = document.getElementById('qbSqlOutput');
      if (qbSqlEl) {
        qbSqlEl.value = sql;
      }
      const qbParamsEl = document.getElementById('qbParamsOutput');
      if (qbParamsEl) {
        qbParamsEl.value = Object.keys(paramsObj).length > 0 ? JSON.stringify(paramsObj, null, 2) : '{}';
      }

      const sqlEl = document.getElementById('epSqlInput');
      if (sqlEl && force) {
        sqlEl.value = sql;
        autoDetectTransactionCheckbox();
      }

      const paramsEl = document.getElementById('epTestParamsInput');
      if (paramsEl && force && Object.keys(paramsObj).length > 0) {
        paramsEl.value = JSON.stringify(paramsObj, null, 2);
      }
    }

    // ================= VISUAL SQL BUILDER ACTIONS =================
    function copyGeneratedSql() {
      const sqlEl = document.getElementById('qbSqlOutput');
      const sql = sqlEl ? sqlEl.value : '';
      if (!sql.trim()) {
        showToast('No SQL generated to copy', 'warning');
        return;
      }
      if (navigator.clipboard && navigator.clipboard.writeText) {
        navigator.clipboard.writeText(sql).then(() => {
          showToast('SQL query copied to clipboard!', 'success');
        }).catch(() => {
          fallbackCopyText(sql);
        });
      } else {
        fallbackCopyText(sql);
      }
    }

    function fallbackCopyText(text) {
      const ta = document.createElement('textarea');
      ta.value = text;
      ta.style.position = 'fixed';
      ta.style.opacity = '0';
      document.body.appendChild(ta);
      ta.select();
      try {
        document.execCommand('copy');
        showToast('SQL query copied to clipboard!', 'success');
      } catch (e) {
        showToast('Failed to copy SQL to clipboard', 'error');
      }
      document.body.removeChild(ta);
    }

    function insertSqlIntoCustomApi() {
      const sql = document.getElementById('qbSqlOutput')?.value || '';
      if (!sql.trim()) {
        showToast('No SQL query generated to insert into Custom API', 'warning');
        return;
      }
      const rawParams = document.getElementById('qbParamsOutput')?.value || '{}';

      // 1. Switch to Custom API view
      switchMainView('endpoints');

      // 2. If no endpoint is currently selected, start new endpoint
      if (!currentCustomEpId) {
        startNewCustomEndpoint();
        const table = qbSelectedTable || 'records';
        const action = qbCurrentAction.toLowerCase();
        const epNameInput = document.getElementById('epNameInput');
        const epPathInput = document.getElementById('epPathInput');
        const epMethodInput = document.getElementById('epMethodInput');

        if (epNameInput && !epNameInput.value) {
          epNameInput.value = `${qbCurrentAction} ${table.charAt(0).toUpperCase() + table.slice(1)}`;
        }
        if (epPathInput && !epPathInput.value) {
          epPathInput.value = `${table}/${action}`;
        }
        if (epMethodInput) {
          if (qbCurrentAction === 'SELECT' || qbCurrentAction === 'COUNT') epMethodInput.value = 'GET';
          else if (qbCurrentAction === 'INSERT') epMethodInput.value = 'POST';
          else if (qbCurrentAction === 'UPDATE') epMethodInput.value = 'PATCH';
          else if (qbCurrentAction === 'DELETE') epMethodInput.value = 'DELETE';
        }
      }

      // 3. Put SQL and params into the Custom API editor
      const sqlInput = document.getElementById('epSqlInput');
      if (sqlInput) {
        sqlInput.value = sql;
        autoDetectTransactionCheckbox();
        sqlInput.focus();
        sqlInput.classList.add('ring-2', 'ring-emerald-500');
        setTimeout(() => sqlInput.classList.remove('ring-2', 'ring-emerald-500'), 1500);
      }

      const paramsInput = document.getElementById('epTestParamsInput');
      if (paramsInput && rawParams.trim()) {
        paramsInput.value = rawParams;
      }

      updateCustomEpUrlPreview();
      showToast('SQL query and parameters inserted into Custom API!', 'success');
    }

    function resetQueryBuilder() {
      qbCurrentAction = 'SELECT';
      qbFilterConditions = [];
      qbJoins = [];
      qbExtraTables = [];
      if (collections.length > 0) {
        qbSelectedTable = currentCollection || collections[0].name;
      }
      const sel = document.getElementById('qbTableSelect');
      if (sel && qbSelectedTable) {
        sel.value = qbSelectedTable;
      }
      setQbAction('SELECT');
      if (qbSelectedTable) {
        onQbTableChange(qbSelectedTable, true);
      }
      const resContainer = document.getElementById('qbResultContainer');
      if (resContainer) resContainer.classList.add('hidden');
      showToast('Visual SQL Builder has been reset', 'info');
    }

    function switchQbResultTab(tab) {
      qbCurrentResultTab = tab;
      const btnJson = document.getElementById('qbResultTab_json');
      const btnTable = document.getElementById('qbResultTab_table');
      const paneJson = document.getElementById('qbResultJsonPane');
      const paneTable = document.getElementById('qbResultTablePane');

      if (tab === 'json') {
        if (btnJson) {
          btnJson.className = 'px-3.5 py-1.5 rounded-lg text-xs font-bold transition flex items-center space-x-1.5 bg-slate-900 text-white shadow-2xs cursor-pointer';
        }
        if (btnTable) {
          btnTable.className = 'px-3.5 py-1.5 rounded-lg text-xs font-semibold text-slate-600 hover:text-slate-900 hover:bg-white transition flex items-center space-x-1.5 cursor-pointer';
        }
        if (paneJson) paneJson.classList.remove('hidden');
        if (paneTable) paneTable.classList.add('hidden');
      } else {
        if (btnJson) {
          btnJson.className = 'px-3.5 py-1.5 rounded-lg text-xs font-semibold text-slate-600 hover:text-slate-900 hover:bg-white transition flex items-center space-x-1.5 cursor-pointer';
        }
        if (btnTable) {
          btnTable.className = 'px-3.5 py-1.5 rounded-lg text-xs font-bold transition flex items-center space-x-1.5 bg-emerald-600 text-white shadow-2xs cursor-pointer';
        }
        if (paneJson) paneJson.classList.add('hidden');
        if (paneTable) paneTable.classList.remove('hidden');
      }
    }
    window.switchQbResultTab = switchQbResultTab;

    function copyQbResultJson() {
      if (!qbLastResultData) {
        showToast('No execution result to copy', 'warning');
        return;
      }
      const text = typeof qbLastResultData === 'string' ? qbLastResultData : JSON.stringify(qbLastResultData, null, 2);
      navigator.clipboard.writeText(text).then(() => {
        showToast('Result JSON copied to clipboard!', 'success');
      }).catch(() => {
        showToast('Failed to copy to clipboard', 'error');
      });
    }
    window.copyQbResultJson = copyQbResultJson;

    async function testGeneratedSqlQuery() {
      const sql = document.getElementById('qbSqlOutput')?.value || '';
      if (!sql.trim()) {
        showToast('No SQL query to execute', 'warning');
        return;
      }

      const resultContainer = document.getElementById('qbResultContainer');
      const resultStatus = document.getElementById('qbResultStatus');
      const resultJsonPre = document.getElementById('qbResultJsonPre');
      const resultTableWrapper = document.getElementById('qbResultTableWrapper');

      if (resultContainer) {
        resultContainer.classList.remove('hidden');
        resultContainer.scrollIntoView({ behavior: 'smooth', block: 'nearest' });
      }
      if (resultStatus) {
        resultStatus.textContent = 'Executing...';
        resultStatus.className = 'text-[10px] font-mono font-bold px-2 py-0.5 rounded bg-amber-100 text-amber-800 animate-pulse';
      }
      if (resultJsonPre) {
        resultJsonPre.textContent = 'Running query against database...';
      }
      if (resultTableWrapper) {
        resultTableWrapper.innerHTML = '<div class="p-6 text-center text-xs text-slate-400 font-mono"><i class="fa-solid fa-spinner fa-spin text-cyan-600 mr-2"></i>Running query against database...</div>';
      }

      // Default tab is always JSON
      switchQbResultTab('json');

      let params = [];
      const rawParams = (document.getElementById('qbParamsOutput')?.value || '').trim();
      if (rawParams && rawParams !== '{}') {
        try {
          const parsed = JSON.parse(rawParams);
          if (Array.isArray(parsed)) {
            params = parsed;
          } else if (typeof parsed === 'object') {
            params = Object.values(parsed);
          }
        } catch (e) {
          if (resultStatus) {
            resultStatus.textContent = 'Invalid JSON Params';
            resultStatus.className = 'text-[10px] font-mono font-bold px-2 py-0.5 rounded bg-rose-100 text-rose-800';
          }
          if (resultJsonPre) {
            resultJsonPre.textContent = `Error parsing Parameters JSON: ${e.message}`;
          }
          if (resultTableWrapper) {
            resultTableWrapper.innerHTML = `<div class="p-4 text-xs font-mono text-rose-600 bg-rose-50">Error parsing Parameters JSON: ${escapeHtml(e.message)}</div>`;
          }
          return;
        }
      }

      const t0 = performance.now();
      try {
        const res = await fetch('/api/sql', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ sql, params, transaction: true })
        });
        const dur = Math.round(performance.now() - t0);
        const data = await res.json();
        qbLastResultData = data;

        if (res.ok) {
          if (resultStatus) {
            resultStatus.textContent = `200 OK (${dur}ms)`;
            resultStatus.className = 'text-[10px] font-mono font-bold px-2 py-0.5 rounded bg-emerald-100 text-emerald-800';
          }
          renderQbQueryResult(data);
        } else {
          if (resultStatus) {
            resultStatus.textContent = `${res.status} Error (${dur}ms)`;
            resultStatus.className = 'text-[10px] font-mono font-bold px-2 py-0.5 rounded bg-rose-100 text-rose-800';
          }
          const errMsg = data.message || JSON.stringify(data, null, 2);
          if (resultJsonPre) {
            resultJsonPre.textContent = errMsg;
          }
          if (resultTableWrapper) {
            resultTableWrapper.innerHTML = `<div class="p-4 text-xs font-mono text-rose-600 bg-rose-50 whitespace-pre-wrap">${escapeHtml(errMsg)}</div>`;
          }
        }
      } catch (err) {
        const dur = Math.round(performance.now() - t0);
        if (resultStatus) {
          resultStatus.textContent = `Error (${dur}ms)`;
          resultStatus.className = 'text-[10px] font-mono font-bold px-2 py-0.5 rounded bg-rose-100 text-rose-800';
        }
        if (resultJsonPre) {
          resultJsonPre.textContent = `Network / Execution Error: ${err.message}`;
        }
        if (resultTableWrapper) {
          resultTableWrapper.innerHTML = `<div class="p-4 text-xs font-mono text-rose-600 bg-rose-50">${escapeHtml(err.message)}</div>`;
        }
      }
    }
    window.testGeneratedSqlQuery = testGeneratedSqlQuery;

    function renderQbQueryResult(data) {
      qbLastResultData = data;
      const resultJsonPre = document.getElementById('qbResultJsonPre');
      const resultTableWrapper = document.getElementById('qbResultTableWrapper');

      // 1. Populate JSON view
      if (resultJsonPre) {
        resultJsonPre.textContent = JSON.stringify(data, null, 2);
      }

      // 2. Populate Table view
      if (resultTableWrapper) {
        const items = data?.items || (Array.isArray(data) ? data : null);
        if (Array.isArray(items)) {
          if (items.length === 0) {
            resultTableWrapper.innerHTML = '<div class="p-6 text-center text-xs text-slate-500 italic bg-white">Query returned 0 rows.</div>';
          } else {
            const headers = Object.keys(items[0] || {});
            let tableHtml = `
              <table class="w-full text-left text-xs text-slate-700 bg-white border-collapse">
                <thead class="bg-slate-100 text-slate-800 font-bold sticky top-0 border-b border-slate-200 text-[11px] font-mono">
                  <tr>
                    ${headers.map(h => `<th class="px-3 py-2 whitespace-nowrap">${escapeHtml(h)}</th>`).join('')}
                  </tr>
                </thead>
                <tbody class="divide-y divide-slate-150 font-mono text-[11px]">
            `;

            items.forEach((row, idx) => {
              tableHtml += `<tr class="${idx % 2 === 0 ? 'bg-white' : 'bg-slate-50/60'} hover:bg-slate-100/70 transition">`;
              headers.forEach(h => {
                const val = row[h];
                let displayVal = val === null || val === undefined ? '<span class="text-slate-400 italic">null</span>' : (typeof val === 'object' ? JSON.stringify(val) : String(val));
                tableHtml += `<td class="px-3 py-1.5 whitespace-nowrap max-w-xs truncate" title="${escapeHtml(String(val))}">${typeof val === 'object' || val === null || val === undefined ? displayVal : escapeHtml(displayVal)}</td>`;
              });
              tableHtml += '</tr>';
            });

            tableHtml += '</tbody></table>';
            resultTableWrapper.innerHTML = tableHtml;
          }
        } else {
          // Mutation result (e.g. rowsAffected, lastInsertId) or summary object
          let summaryRows = '';
          if (typeof data === 'object' && data !== null) {
            for (const [k, v] of Object.entries(data)) {
              summaryRows += `
                <tr class="hover:bg-slate-50 border-b border-slate-100 last:border-b-0">
                  <td class="px-3 py-2 font-bold text-slate-600 bg-slate-50 border-r border-slate-200 w-1/3">${escapeHtml(k)}</td>
                  <td class="px-3 py-2 text-slate-900">${escapeHtml(typeof v === 'object' ? JSON.stringify(v) : String(v))}</td>
                </tr>
              `;
            }
          }
          resultTableWrapper.innerHTML = `
            <div class="p-4 bg-white space-y-3">
              <div class="flex items-center space-x-2 text-emerald-700 font-bold text-xs">
                <i class="fa-solid fa-circle-check"></i>
                <span>Statement Executed Successfully</span>
              </div>
              <table class="w-full text-left text-xs border border-slate-200 rounded-lg overflow-hidden font-mono">
                <tbody>${summaryRows || '<tr><td class="p-3 text-slate-500">Success</td></tr>'}</tbody>
              </table>
            </div>
          `;
        }
      }

      // Default active tab is JSON
      switchQbResultTab('json');
    }
    window.renderQbQueryResult = renderQbQueryResult;

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
        case 'own_create':
          const targetCol = currentCollection || 'banks';
          if (nameEl && !nameEl.value) nameEl.value = `My Own ${targetCol} Records`;
          if (pathEl && !pathEl.value) pathEl.value = `${targetCol}/my-records`;
          if (methodEl) methodEl.value = 'GET';
          if (txEl) txEl.checked = false;
          const authRuleInput = document.getElementById('epAuthRuleInput');
          if (authRuleInput) {
            authRuleInput.value = "@request.auth.id != ''";
            updateRoleDropdownUI('epAuthRuleInput');
          }
          sqlEl.value = `-- ================================================================\n-- 👤 Own Create / Record Ownership Filter\n-- Only returns records created by the currently logged-in user.\n-- Evaluates @request.auth.id from the caller's JWT token.\n-- ================================================================\nSELECT *\nFROM \`${targetCol}\`\nWHERE \`created_by\` = @request.auth.id\nORDER BY \`id\` DESC;`;
          if (paramsEl) paramsEl.value = '{}';
          break;

        case 'parent_child_last_id':
          if (nameEl && !nameEl.value) nameEl.value = 'Create Order with Items (Auto :last_insert_id)';
          if (pathEl && !pathEl.value) pathEl.value = 'orders/create-with-items';
          if (methodEl) methodEl.value = 'POST';
          if (txEl) txEl.checked = true;
          sqlEl.value = `-- ================================================================\n-- 🔗 Parent + Child Transaction with Auto :last_insert_id\n-- 1. Insert parent record (Order)\n-- 2. Insert child record (Order Items) using the auto-generated\n--    :last_insert_id from statement 1 above.\n-- All operations run inside an atomic database transaction.\n-- ================================================================\n\n-- 1. Insert into Header / Parent Table (orders)\nINSERT INTO \`orders\` (\`customer_name\`, \`total_amount\`, \`status\`)\nVALUES (:customer_name, :total_amount, 'completed');\n\n-- 2. Insert into Detail / Child Table (order_items) using :last_insert_id\nINSERT INTO \`order_items\` (\`order_id\`, \`product_name\`, \`quantity\`, \`price\`)\nVALUES (:last_insert_id, :product_name, :quantity, :price);`;
          if (paramsEl) {
            paramsEl.value = JSON.stringify({
              customer_name: "Sarah Connor",
              total_amount: 199.99,
              product_name: "Cyberdyne Model 101 Manual",
              quantity: 1,
              price: 199.99
            }, null, 2);
          }
          break;

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
          sqlEl.value = `-- ================================================================\n-- 🛒 POS / Cashier Checkout (Multi-Table Atomic Transaction)\n-- All queries below run inside a single database transaction.\n-- If any error occurs or stock is insufficient, all changes are ROLLED BACK.\n-- ================================================================\n\n-- 1. Insert into Header Table (orders / sales)\nINSERT INTO \`orders\` (\`customer_name\`, \`total_amount\`, \`payment_method\`, \`cashier_id\`, \`status\`)\nVALUES (:customer_name, :total_amount, :payment_method, :cashier_id, 'completed');\n\n-- 2. Insert into Detail Table (order_items) using auto :last_insert_id\nINSERT INTO \`order_items\` (\`order_id\`, \`product_id\`, \`product_name\`, \`quantity\`, \`price\`, \`subtotal\`)\nVALUES (:last_insert_id, :product_id, :product_name, :quantity, :price, :subtotal);\n\n-- 3. Deduct product inventory stock in Products Table\nUPDATE \`products\`\nSET \`stock\` = \`stock\` - :quantity\nWHERE \`id\` = :product_id AND \`stock\` >= :quantity;`;
          if (paramsEl) {
            paramsEl.value = JSON.stringify({
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
              details: "Created via Custom API"
            }, null, 2);
          }
          break;

        case 'dashboard_summary':
          if (nameEl && !nameEl.value) nameEl.value = 'Dashboard Metrics & Summary';
          if (pathEl && !pathEl.value) pathEl.value = 'dashboard/summary';
          if (methodEl) methodEl.value = 'GET';
          if (txEl) txEl.checked = false;
          sqlEl.value = `-- ================================================================\n-- 📊 Dashboard Metrics: Total Records, Status Counts & Revenue\n-- Use LIMIT 1 to produce a direct single JSON object { ... }.\n-- ================================================================\nSELECT\n  COUNT(*) AS \`total_records\`,\n  COUNT(CASE WHEN \`status\` = 'active' OR \`status\` = 'completed' THEN 1 END) AS \`active_count\`,\n  COUNT(CASE WHEN \`status\` = 'pending' THEN 1 END) AS \`pending_count\`,\n  COALESCE(SUM(\`total_amount\`), 0) AS \`total_revenue\`,\n  COALESCE(AVG(\`total_amount\`), 0) AS \`average_amount\`,\n  MAX(\`created_at\`) AS \`latest_activity\`\nFROM \`${table}\` LIMIT 1;`;
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

      // Also sync into Visual SQL Builder if present
      const qbSql = document.getElementById('qbSqlOutput');
      if (qbSql && sqlEl) {
        qbSql.value = sqlEl.value;
      }
      const qbParams = document.getElementById('qbParamsOutput');
      if (qbParams && paramsEl) {
        qbParams.value = paramsEl.value;
      }

      if (typeof updateCustomEpUrlPreview === 'function') {
        updateCustomEpUrlPreview();
      }
      if (typeof autoDetectTransactionCheckbox === 'function') {
        autoDetectTransactionCheckbox();
      }
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

      // Auto-save new custom prefix if user entered one
      const prefixSelect = document.getElementById('epPrefixSelect');
      const customPrefixInput = document.getElementById('epCustomPrefixInput');
      if (prefixSelect && (prefixSelect.value === 'custom' || prefixSelect.value === '__custom__')) {
        const rawCustom = (customPrefixInput?.value || '').trim();
        if (rawCustom) {
          saveCustomPrefix(rawCustom);
        }
      }

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
        const currentPfx = getSelectedPrefix();
        const currentSlug = (document.getElementById('epPathInput')?.value || '').trim().replace(/^\/+/, '');
        saveEndpointPathMeta(saved.id, currentPfx, currentSlug);
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
      if (!confirm('Delete this custom API endpoint?')) return;
      const res = await fetch(`/api/endpoints/${currentCustomEpId}`, { method: 'DELETE' });
      if (res.ok) {
        deleteEndpointPathMeta(currentCustomEpId);
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
      const tabs = ['app', 'roles', 'admins', 'backups', 'email', 'cron'];
      tabs.forEach(t => {
        const pane = document.getElementById(`settingsPane_${t}`);
        if (pane) pane.classList.toggle('hidden', t !== tab);

        const card = document.getElementById(`setTabContainer_${t}`);
        const icon = document.getElementById(`setTabIcon_${t}`);
        const sub = document.getElementById(`setTabSub_${t}`);
        const badge = document.getElementById(`setTabBadge_${t}`);
        const isSelected = t === tab;

        if (card) {
          card.className = isSelected
            ? "group relative flex items-center rounded-2xl transition border bg-emerald-600 text-white border-emerald-600 shadow-sm font-bold"
            : "group relative flex items-center rounded-2xl transition border bg-white hover:bg-emerald-50/70 text-slate-700 border-slate-100 font-semibold";
        }
        if (icon) {
          icon.className = isSelected
            ? "w-8 h-8 rounded-xl flex items-center justify-center shrink-0 bg-white/20 text-white"
            : "w-8 h-8 rounded-xl flex items-center justify-center shrink-0 bg-emerald-50 text-emerald-700";
        }
        if (sub) {
          sub.className = isSelected ? "text-[10px] text-emerald-100" : "text-[10px] text-slate-400";
        }
        if (badge) {
          badge.className = isSelected
            ? "px-2 py-0.5 rounded-lg text-[9px] font-mono font-bold bg-white/20 text-white"
            : "px-2 py-0.5 rounded-lg text-[9px] font-mono font-bold bg-emerald-50 text-emerald-700";
        }

        const btn = document.getElementById(`setTab_${t}`);
        if (btn) {
          btn.className = isSelected
            ? "w-full text-left px-3 py-2 rounded-xl text-xs font-bold bg-emerald-600 text-white shadow-2xs flex items-center space-x-2.5 transition shrink-0"
            : "w-full text-left px-3 py-2 rounded-xl text-xs font-semibold text-slate-700 hover:bg-slate-200/70 flex items-center space-x-2.5 transition shrink-0";
        }
      });
      if (tab === 'roles') renderRolesSettings();
      if (tab === 'backups') loadBackups();
      if (tab === 'admins') loadAdminAccounts();
      if (tab === 'cron') loadCronJobs();
    }

    function filterSettingsTabs() {
      const q = (document.getElementById('settingsSearchInput')?.value || '').toLowerCase().trim();
      const tabs = ['app', 'roles', 'admins', 'backups', 'email'];
      tabs.forEach(t => {
        const c = document.getElementById(`setTabContainer_${t}`);
        if (!c) return;
        const text = c.textContent.toLowerCase();
        c.classList.toggle('hidden', q !== '' && !text.includes(q));
      });
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
          <div class="group relative flex items-center rounded-2xl transition border ${isActive ? 'bg-slate-200/90 text-slate-900 border-slate-300 shadow-xs font-bold' : 'bg-white hover:bg-slate-100 text-slate-700 border-slate-200/60 font-semibold'}">
            <button onclick="selectCollection('${c.name}')" class="flex-1 flex items-center justify-between p-2.5 text-xs min-w-0">
              <div class="flex items-center space-x-2.5 truncate">
                <span class="w-8 h-8 rounded-xl flex items-center justify-center shrink-0 ${isActive ? 'bg-white text-slate-800 shadow-2xs' : 'bg-slate-100 text-slate-600'}">
                  <i class="fa-solid ${icon} text-[11px]"></i>
                </span>
                <div class="text-left truncate">
                  <div class="truncate font-mono">${c.name}</div>
                  <div class="text-[10px] ${isActive ? 'text-slate-500 font-medium' : 'text-slate-400'}">${c.type} &bull; ${fieldCount} columns</div>
                </div>
              </div>
              <div class="flex items-center space-x-1 shrink-0 ml-1">
                ${isRel ? `<span class="px-1.5 py-0.5 rounded-md text-[9px] font-mono ${isActive ? 'bg-white text-teal-800 shadow-2xs border border-slate-200' : 'bg-teal-50 text-teal-700'}" title="Auto Relations Active"><i class="fa-solid fa-link"></i></span>` : ''}
                ${isRealtime
                  ? `<span class="px-2 py-1 rounded-lg text-[10px] font-mono ${isActive ? 'bg-white text-emerald-700 shadow-2xs border border-slate-200' : 'bg-emerald-50 text-emerald-700'}" title="Realtime SSE Active"><i class="fa-solid fa-bolt"></i></span>`
                  : `<span class="px-1.5 py-0.5 rounded-md text-[9px] font-mono ${isActive ? 'bg-slate-300/80 text-slate-600' : 'bg-slate-100 text-slate-400'}" title="Realtime Inactive">off</span>`}
              </div>
            </button>
            <button onclick="selectCollection('${c.name}'); switchWorkspaceSubView('settings');" class="p-2 mr-1 rounded-lg ${isActive ? 'text-slate-600 hover:text-slate-900 hover:bg-slate-300/60' : 'text-slate-400 hover:bg-slate-100 hover:text-slate-700'} transition" title="Settings for table ${c.name}">
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
      updateSelectAllCheckboxState();
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
          { name: 'id', type: 'text', sql: 'VARCHAR(255) PK', desc: 'User record unique Primary Key (Auto-generated)', icon: 'fa-solid fa-fingerprint', isBlameable: false },
          { name: 'email', type: 'email', sql: 'VARCHAR(255) UNIQUE', desc: 'Login credential email (Required & Unique)', icon: 'fa-solid fa-envelope', isBlameable: false },
          { name: 'password', type: 'password', sql: 'VARCHAR(255) / TEXT', desc: 'Bcrypt-hashed encrypted password', icon: 'fa-solid fa-key', isBlameable: false },
          { name: 'name', type: 'text', sql: 'VARCHAR(255)', desc: 'User display full name', icon: 'fa-solid fa-id-card', isBlameable: false },
          { name: 'username', type: 'text', sql: 'VARCHAR(50)', desc: 'Unique handle / user login username', icon: 'fa-solid fa-at', isBlameable: false },
          { name: 'role', type: 'text', sql: 'VARCHAR(50)', desc: 'Access authorization role (default: "user")', icon: 'fa-solid fa-shield-halved', isBlameable: false },
          { name: 'status', type: 'text', sql: 'VARCHAR(20)', desc: 'User account status (default: "active")', icon: 'fa-solid fa-circle-check', isBlameable: false },
          { name: 'verified', type: 'bool', sql: 'INTEGER', desc: 'User email verification status (0 / 1)', icon: 'fa-solid fa-badge-check', isBlameable: false },
          { name: 'tokenKey', type: 'text', sql: 'VARCHAR(255)', desc: 'Session & auth token encryption key', icon: 'fa-solid fa-lock', isBlameable: false },
          { name: 'created_at', type: 'number', sql: 'BIGINT / TIMESTAMP', desc: 'Timestamp when the account was first created', icon: 'fa-solid fa-calendar-plus', isBlameable: false },
          { name: 'updated_at', type: 'number', sql: 'BIGINT / TIMESTAMP', desc: 'Timestamp when the account profile was modified', icon: 'fa-solid fa-clock-rotate-left', isBlameable: false },
          { 
            name: 'created_by', 
            type: 'text', 
            sql: 'VARCHAR(255)', 
            desc: isBlameable ? 'Creator user/admin ID (Blameable active)' : 'Creator user/admin ID (Disabled — click badge to enable in Table Options)', 
            icon: 'fa-solid fa-user-check', 
            isBlameable: true 
          },
          { 
            name: 'updated_by', 
            type: 'text', 
            sql: 'VARCHAR(255)', 
            desc: isBlameable ? 'Editor user/admin ID (Blameable active)' : 'Editor user/admin ID (Disabled — click badge to enable in Table Options)', 
            icon: 'fa-solid fa-user-pen', 
            isBlameable: true 
          }
        ];
      } else {
        // Base Table
        fields = [
          { name: 'id', type: 'text', sql: 'VARCHAR(255) PK', desc: 'Unique record Primary Key (Auto-generated)', icon: 'fa-solid fa-fingerprint', isBlameable: false },
          { name: 'created_at', type: 'number', sql: 'BIGINT / TIMESTAMP', desc: 'Auto Unix timestamp when record was inserted', icon: 'fa-solid fa-calendar-plus', isBlameable: false },
          { name: 'updated_at', type: 'number', sql: 'BIGINT / TIMESTAMP', desc: 'Auto Unix timestamp when record was updated', icon: 'fa-solid fa-clock-rotate-left', isBlameable: false },
          { 
            name: 'created_by', 
            type: 'text', 
            sql: 'VARCHAR(255)', 
            desc: isBlameable ? 'Record creator user ID (Blameable active, skipped if unauthenticated)' : 'Record creator user ID (Disabled — click badge to enable in Table Options)', 
            icon: 'fa-solid fa-user-check', 
            isBlameable: true 
          },
          { 
            name: 'updated_by', 
            type: 'text', 
            sql: 'VARCHAR(255)', 
            desc: isBlameable ? 'Record editor user ID (Blameable active, skipped if unauthenticated)' : 'Record editor user ID (Disabled — click badge to enable in Table Options)', 
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
            badgeHtml = `<span class="inline-flex items-center text-[10px] text-emerald-700 font-semibold px-2 py-0.5 rounded-full bg-emerald-50 border border-emerald-200 shrink-0 ml-1.5" title="Blameable behaviour active"><i class="fa-solid fa-circle-check text-[9px] mr-1 text-emerald-600"></i>Blameable: ON</span>`;
          } else {
            badgeHtml = `<button type="button" onclick="switchColSettingsTab('options')" class="inline-flex items-center text-[10px] text-slate-500 hover:text-indigo-700 font-semibold px-2 py-0.5 rounded-full bg-slate-100 hover:bg-indigo-50 border border-slate-200 hover:border-indigo-200 shrink-0 ml-1.5 transition cursor-pointer" title="Click to enable in Table Options tab"><i class="fa-solid fa-toggle-off text-[9px] mr-1 text-slate-400"></i>Blameable: OFF</button>`;
          }
        } else {
          badgeHtml = `<span class="inline-flex items-center text-[10px] text-emerald-700 font-semibold px-1.5 py-0.5 rounded bg-emerald-50 border border-emerald-200 shrink-0 ml-1.5" title="This default field is automatically managed by PadiBase"><i class="fa-solid fa-lock text-[8px] mr-1 text-emerald-600"></i>Default</span>`;
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
        colContainer.innerHTML = `<div class="py-6 text-center text-xs text-slate-400 border border-dashed border-slate-300 rounded-2xl">No custom columns added yet. Use the Add Column buttons below to add application data columns.</div>`;
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
              <span>Blameable disabled. <code class="font-mono text-slate-700">created_by</code> and <code class="font-mono text-slate-700">updated_by</code> fields are not automatically populated.</span>
            </div>
          `;
        } else if (hasCreatedBy && hasUpdatedBy) {
          bNotice.innerHTML = `
            <div class="p-2.5 rounded-lg bg-emerald-500/10 border border-emerald-500/30 text-emerald-800 text-xs flex items-center space-x-2">
              <i class="fa-solid fa-circle-check text-emerald-600"></i>
              <span>Fields <code class="font-mono font-bold bg-emerald-100/80 px-1 rounded">created_by</code> &amp; <code class="font-mono font-bold bg-emerald-100/80 px-1 rounded">updated_by</code> detected and active.</span>
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
                  <span class="font-bold">Notice:</span> Field <code class="font-mono font-bold bg-amber-100/80 px-1 rounded">${missing.join(' & ')}</code> is not yet present in this table schema.
                  <p class="text-[11px] text-amber-800 mt-0.5">PadiBase will automatically create them when you save these settings.</p>
                </div>
              </div>
              <button type="button" onclick="autoAddBlameableToSchema()" class="px-2.5 py-1 rounded-md bg-amber-600 hover:bg-amber-700 text-white text-[11px] font-bold transition flex items-center space-x-1 shrink-0">
                <i class="fa-solid fa-plus text-[10px]"></i>
                <span>Add to Table Schema</span>
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
              <span>Timestamp fields <code class="font-mono font-bold bg-emerald-100/80 px-1 rounded">created_at</code> &amp; <code class="font-mono font-bold bg-emerald-100/80 px-1 rounded">updated_at</code> detected and active.</span>
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
                  <span class="font-bold">Notice:</span> Field <code class="font-mono font-bold bg-sky-100/80 px-1 rounded">${missingT.join(' & ')}</code> is not yet in this table.
                  <p class="text-[11px] text-sky-800 mt-0.5">PadiBase automatically populates timestamps if these fields exist in the table.</p>
                </div>
              </div>
              <button type="button" onclick="autoAddTimestampsToSchema()" class="px-2.5 py-1 rounded-md bg-sky-600 hover:bg-sky-700 text-white text-[11px] font-bold transition flex items-center space-x-1 shrink-0">
                <i class="fa-solid fa-plus text-[10px]"></i>
                <span>Add to Table Schema</span>
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
      showToast(`Successfully added ${added} blameable column(s) to table schema.`);
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
      showToast(`Successfully added ${added} timestamp column(s) (created_at & updated_at) to table schema.`);
    }

    function onInlineBlameableToggleChange(checked) {
      updateOptionsFieldNotices();
      renderInlineDefaultFields();
      if (checked) {
        const hasCreatedBy = inlineSchemaFields.some(f => f.name && f.name.toLowerCase() === 'created_by');
        const hasUpdatedBy = inlineSchemaFields.some(f => f.name && f.name.toLowerCase() === 'updated_by');
        if (!hasCreatedBy || !hasUpdatedBy) {
          showToast('Blameable enabled. created_by & updated_by columns will be ensured upon saving.', 'info');
        } else {
          showToast('Blameable active.', 'success');
        }
      } else {
        showToast('Blameable disabled.', 'info');
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

        const res = await fetch(`/api/${currentCollection}?${params.toString()}`);
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

      const isAllSelected = items.length > 0 && items.every(r => selectedRecordIds.has(r.id));
      const hasSomeSelected = items.some(r => selectedRecordIds.has(r.id));
      const totalColCount = 3 + schemaCols.length;

      const sortIcon = (field) => {
        if (currentSort === field) return '<i class="fa-solid fa-caret-up ml-1 text-emerald-600"></i>';
        if (currentSort === `-${field}`) return '<i class="fa-solid fa-caret-down ml-1 text-emerald-600"></i>';
        return '';
      };

      thead.innerHTML = `
        <th class="py-3 px-3 w-8 border-r border-slate-200/60 text-center">
          <input type="checkbox" id="selectAllCheckbox" onchange="toggleSelectAll(this.checked)" ${isAllSelected ? 'checked' : ''} class="rounded border-slate-300">
        </th>
        <th class="py-3 px-3 w-20 border-r border-slate-200/60 text-center whitespace-nowrap">
          <span class="text-[10px] font-bold text-slate-500 uppercase tracking-wider">Action</span>
        </th>
        <th onclick="toggleSort('id')" class="py-3 px-4 border-r border-slate-200/60 cursor-pointer hover:bg-emerald-50/50 transition whitespace-nowrap">
          <i class="fa-solid fa-fingerprint text-[10px] text-emerald-600 mr-1"></i> id ${sortIcon('id')}
        </th>
        ${schemaCols.map(c => `
          <th onclick="toggleSort('${c.name}')" class="py-3 px-4 border-r border-slate-200/60 cursor-pointer hover:bg-emerald-50/50 transition whitespace-nowrap">
            <i class="${FIELD_ICONS[c.type] || 'fa-solid fa-font'} text-[10px] text-emerald-600 mr-1"></i> ${c.name} ${sortIcon(c.name)}
          </th>
        `).join('')}
      `;

      const selectAllEl = document.getElementById('selectAllCheckbox');
      if (selectAllEl) {
        selectAllEl.checked = isAllSelected;
        selectAllEl.indeterminate = !isAllSelected && hasSomeSelected;
      }

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
              <td onclick="event.stopPropagation()" class="py-2.5 px-3 border-r border-slate-100 text-center">
                <input type="checkbox" ${checked} onchange="toggleSelectRecord('${item.id}', this.checked)" class="rounded border-slate-300">
              </td>
              <td onclick="event.stopPropagation()" class="py-2.5 px-3 border-r border-slate-100 text-center whitespace-nowrap">
                <button type="button" onclick="openRecordDrawer('${item.id}')" class="px-2.5 py-1 rounded-lg bg-amber-400 hover:bg-amber-500 active:scale-95 text-slate-900 font-bold text-[11px] shadow-xs hover:shadow transition inline-flex items-center space-x-1.5 cursor-pointer border border-amber-300" title="Edit Record">
                  <i class="fa-solid fa-pen-to-square text-[10px] text-amber-950"></i>
                  <span>Edit</span>
                </button>
              </td>
              <td onclick="event.stopPropagation(); copyToClipboard('${item.id}')" class="py-2.5 px-4 border-r border-slate-100 whitespace-nowrap">
                <span class="px-2.5 py-0.5 rounded-lg bg-emerald-50 group-hover:bg-emerald-100 text-emerald-800 border border-emerald-200/70 font-semibold text-[11px] inline-flex items-center">
                  ${item.id} <i class="fa-regular fa-copy ml-1.5 text-[9px] opacity-50"></i>
                </span>
              </td>
              ${schemaCols.map(c => `<td class="py-2.5 px-4 border-r border-slate-100 max-w-[240px] truncate">${formatCell(item[c.name], c.type, item)}</td>`).join('')}
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
        const url = `/api/files/${currentCollection}/${recId}/${val}${fileTokenQuery()}`;
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

    function updateSelectAllCheckboxState() {
      const selectAllCb = document.getElementById('selectAllCheckbox');
      if (!selectAllCb) return;
      const count = currentRecords.length;
      if (count === 0) {
        selectAllCb.checked = false;
        selectAllCb.indeterminate = false;
        return;
      }
      const selectedCount = currentRecords.filter(r => selectedRecordIds.has(r.id)).length;
      if (selectedCount === count) {
        selectAllCb.checked = true;
        selectAllCb.indeterminate = false;
      } else if (selectedCount > 0) {
        selectAllCb.checked = false;
        selectAllCb.indeterminate = true;
      } else {
        selectAllCb.checked = false;
        selectAllCb.indeterminate = false;
      }
    }

    function toggleSelectAll(checked) {
      if (checked) {
        currentRecords.forEach(r => selectedRecordIds.add(r.id));
      } else {
        selectedRecordIds.clear();
      }
      // Langsung update checkbox baris di DOM tanpa re-fetch lambat
      const rowCheckboxes = document.querySelectorAll('#recordsBody input[type="checkbox"]');
      rowCheckboxes.forEach(cb => { cb.checked = checked; });
      updateSelectAllCheckboxState();
      updateBulkDeleteBar();
    }

    function toggleSelectRecord(id, checked) {
      if (checked) selectedRecordIds.add(id);
      else selectedRecordIds.delete(id);
      updateSelectAllCheckboxState();
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
        await fetch(`/api/${currentCollection}/${id}`, { method: 'DELETE' });
      }
      selectedRecordIds.clear();
      updateSelectAllCheckboxState();
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
          const res = await fetch(`/api/${currentCollection}/${recordId}`);
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
            const fileUrl = `/api/files/${currentCollection}/${editingRecordId}/${currentFile}${fileTokenQuery()}`;
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
                          <a href="${fileUrl}&download=1" target="_blank" class="text-[11px] font-semibold text-indigo-700 hover:underline flex items-center space-x-1"><i class="fa-solid fa-download text-[10px]"></i><span>Download</span></a>
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
        ? `/api/${currentCollection}/${editingRecordId}`
        : `/api/${currentCollection}`;
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

      if (dlBtn) dlBtn.href = `${url}&download=1`;
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
      const res = await fetch(`/api/${currentCollection}/${editingRecordId}`, { method: 'DELETE' });
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
          { name: 'id', type: 'text', sql: 'VARCHAR(255) PK', desc: 'User record unique Primary Key (Auto-generated)', icon: 'fa-solid fa-fingerprint', isBlameable: false },
          { name: 'email', type: 'email', sql: 'VARCHAR(255) UNIQUE', desc: 'Login credential email (Required & Unique)', icon: 'fa-solid fa-envelope', isBlameable: false },
          { name: 'password', type: 'password', sql: 'VARCHAR(255) / TEXT', desc: 'Bcrypt-hashed encrypted password', icon: 'fa-solid fa-key', isBlameable: false },
          { name: 'name', type: 'text', sql: 'VARCHAR(255)', desc: 'User display full name', icon: 'fa-solid fa-id-card', isBlameable: false },
          { name: 'username', type: 'text', sql: 'VARCHAR(50)', desc: 'Unique handle / user login username', icon: 'fa-solid fa-at', isBlameable: false },
          { name: 'role', type: 'text', sql: 'VARCHAR(50)', desc: 'Access authorization role (default: "user")', icon: 'fa-solid fa-shield-halved', isBlameable: false },
          { name: 'status', type: 'text', sql: 'VARCHAR(20)', desc: 'User account status (default: "active")', icon: 'fa-solid fa-circle-check', isBlameable: false },
          { name: 'verified', type: 'bool', sql: 'INTEGER', desc: 'User email verification status (0 / 1)', icon: 'fa-solid fa-badge-check', isBlameable: false },
          { name: 'tokenKey', type: 'text', sql: 'VARCHAR(255)', desc: 'Session & auth token encryption key', icon: 'fa-solid fa-lock', isBlameable: false },
          { name: 'created_at', type: 'number', sql: 'BIGINT / TIMESTAMP', desc: 'Timestamp when the account was first created', icon: 'fa-solid fa-calendar-plus', isBlameable: false },
          { name: 'updated_at', type: 'number', sql: 'BIGINT / TIMESTAMP', desc: 'Timestamp when the account profile was modified', icon: 'fa-solid fa-clock-rotate-left', isBlameable: false },
          { 
            name: 'created_by', 
            type: 'text', 
            sql: 'VARCHAR(255)', 
            desc: isBlameable ? 'Creator user/admin ID (Blameable active)' : 'Creator user/admin ID (Blameable disabled)', 
            icon: 'fa-solid fa-user-check', 
            isBlameable: true 
          },
          { 
            name: 'updated_by', 
            type: 'text', 
            sql: 'VARCHAR(255)', 
            desc: isBlameable ? 'Editor user/admin ID (Blameable active)' : 'Editor user/admin ID (Blameable disabled)', 
            icon: 'fa-solid fa-user-pen', 
            isBlameable: true 
          }
        ];
      } else {
        // Base Table
        fields = [
          { name: 'id', type: 'text', sql: 'VARCHAR(255) PK', desc: 'Unique record Primary Key (Auto-generated)', icon: 'fa-solid fa-fingerprint', isBlameable: false },
          { name: 'created_at', type: 'number', sql: 'BIGINT / TIMESTAMP', desc: 'Auto Unix timestamp when record was inserted', icon: 'fa-solid fa-calendar-plus', isBlameable: false },
          { name: 'updated_at', type: 'number', sql: 'BIGINT / TIMESTAMP', desc: 'Auto Unix timestamp when record was updated', icon: 'fa-solid fa-clock-rotate-left', isBlameable: false },
          { 
            name: 'created_by', 
            type: 'text', 
            sql: 'VARCHAR(255)', 
            desc: isBlameable ? 'Record creator user ID (Blameable active, skipped if unauthenticated)' : 'Record creator user ID (Blameable disabled — check Blameable above)', 
            icon: 'fa-solid fa-user-check', 
            isBlameable: true 
          },
          { 
            name: 'updated_by', 
            type: 'text', 
            sql: 'VARCHAR(255)', 
            desc: isBlameable ? 'Record editor user ID (Blameable active, skipped if unauthenticated)' : 'Record editor user ID (Blameable disabled — check Blameable above)', 
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
          badgeHtml = `<span class="inline-flex items-center text-[10px] text-emerald-700 font-semibold px-1.5 py-0.5 rounded bg-emerald-50 border border-emerald-200 shrink-0 ml-1.5" title="Automatically populated and managed by PadiBase"><i class="fa-solid fa-lock text-[8px] mr-1 text-emerald-600"></i>Auto</span>`;
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
          alertEl.textContent = `Column "${dupField.name}" is an auto-generated system field managed by PadiBase. You do not need to add it to Custom Columns.`;
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

      // Only show Upload File endpoint tab if this table actually has column(s) of type 'file'
      const hasFileFields = Boolean(col && Array.isArray(col.schema) && col.schema.some(f => f.type === 'file'));
      const fileTab = document.getElementById('apiTabFile') || document.querySelector('#apiPreviewTabs button[data-api-tab="file"]');
      if (fileTab) fileTab.classList.toggle('hidden', !hasFileFields);

      if (!hasFileFields && currentSelectedApiTab === 'file') {
        currentSelectedApiTab = 'list';
      }

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

      const fileField = (col.schema || []).find(f => f.type === 'file')?.name || 'file';

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
          path: `/api/${cName}?page=1&perPage=30`,
          statusCode: '200 OK',
          js: `const res = await fetch('${baseUrl}/api/${cName}?page=1&perPage=30');\nconst { item, meta, debug } = await res.json();\nconsole.log(item, meta);`,
          curl: `curl -X GET "${baseUrl}/api/${cName}?page=1&perPage=30"`
        },
        all: {
          title: `Fetch All Records Without Pagination (${cName})`,
          desc: `Retrieve rows from ${cName} via standard clean Padi Rust route (/api/${cName}/all). Capped at 5,000 max records. Optional query: ?limit=500 (max: 5000), ?offset=0, ?sort_by=id&order=desc.`,
          method: 'GET',
          badge: 'bg-emerald-100 text-emerald-800',
          path: `/api/${cName}/all?limit=5000&sort_by=id&order=desc`,
          statusCode: '200 OK',
          js: `const res = await fetch('${baseUrl}/api/${cName}/all?limit=5000&sort_by=id&order=desc');\nconst { item, debug } = await res.json();\nconsole.log(item);`,
          curl: `curl -X GET "${baseUrl}/api/${cName}/all?limit=5000&sort_by=id&order=desc"`
        },
        view: {
          title: `Fetch Single Record by ID (${cName})`,
          desc: `Retrieve single record details from ${cName} using ID.`,
          method: 'GET',
          badge: 'bg-emerald-100 text-emerald-800',
          path: `/api/${cName}/RECORD_ID`,
          statusCode: '200 OK',
          js: `const res = await fetch('${baseUrl}/api/${cName}/RECORD_ID');\nconst { item } = await res.json();\nconsole.log(item);`,
          curl: `curl -X GET "${baseUrl}/api/${cName}/RECORD_ID"`
        },
        create: {
          title: `Create New Record (${cName})`,
          desc: `Insert a new record into table ${cName}.`,
          method: 'POST',
          badge: 'bg-sky-100 text-sky-800',
          path: `/api/${cName}`,
          statusCode: '201 Created',
          js: `const res = await fetch('${baseUrl}/api/${cName}', {\n  method: 'POST',\n  headers: { 'Content-Type': 'application/json' },\n  body: JSON.stringify(${JSON.stringify(sampleObj, null, 2)})\n});\nconst { item } = await res.json();`,
          curl: `curl -X POST "${baseUrl}/api/${cName}" \\\n  -H "Content-Type: application/json" \\\n  -d '${JSON.stringify(sampleObj, null, 2)}'`
        },
        update: {
          title: `Update Record (${cName})`,
          desc: `Modify some or all fields of an existing record by ID.`,
          method: 'PATCH',
          badge: 'bg-amber-100 text-amber-800',
          path: `/api/${cName}/RECORD_ID`,
          statusCode: '200 OK',
          js: `const res = await fetch('${baseUrl}/api/${cName}/RECORD_ID', {\n  method: 'PATCH',\n  headers: { 'Content-Type': 'application/json' },\n  body: JSON.stringify(${JSON.stringify(sampleObj, null, 2)})\n});\nconst { item } = await res.json();`,
          curl: `curl -X PATCH "${baseUrl}/api/${cName}/RECORD_ID" \\\n  -H "Content-Type: application/json" \\\n  -d '${JSON.stringify(sampleObj, null, 2)}'`
        },
        delete: {
          title: `Delete Record (${cName})`,
          desc: `Remove a record from table ${cName} by ID.`,
          method: 'DELETE',
          badge: 'bg-rose-100 text-rose-800',
          path: `/api/${cName}/RECORD_ID`,
          statusCode: '200 OK',
          js: `const res = await fetch('${baseUrl}/api/${cName}/RECORD_ID', {\n  method: 'DELETE'\n});\nconst { item } = await res.json();`,
          curl: `curl -X DELETE "${baseUrl}/api/${cName}/RECORD_ID"`
        },
        realtime: {
          title: `Subscribe to Realtime SSE Stream (${cName})`,
          desc: `Listen for live data changes when Realtime is enabled on table ${cName}.`,
          method: 'GET',
          badge: 'bg-purple-100 text-purple-800',
          path: `/api/realtime`,
          statusCode: '200 OK (text/event-stream)',
          js: `const evtSource = new EventSource('${baseUrl}/api/realtime');\nevtSource.onmessage = (event) => {\n  const data = JSON.parse(event.data);\n  if (data.table === '${cName}') {\n    console.log('Change event:', data.action, data.record);\n  }\n};`,
          curl: `curl -N "${baseUrl}/api/realtime"`
        },
        file: {
          title: `Upload File / Image (${cName})`,
          desc: `Upload files via multipart/form-data to column "${fileField}" and link them automatically to the record.`,
          method: 'POST',
          badge: 'bg-indigo-100 text-indigo-800',
          path: `/api/files/${cName}/RECORD_ID`,
          statusCode: '200 OK',
          js: `const formData = new FormData();\nformData.append('${fileField}', fileInputElement.files[0]);\n\nconst res = await fetch('${baseUrl}/api/files/${cName}/RECORD_ID', {\n  method: 'POST',\n  body: formData\n});\nconst result = await res.json();`,
          curl: `curl -X POST "${baseUrl}/api/files/${cName}/RECORD_ID" \\\n  -F "${fileField}=@/path/to/my_image.png"`
        },
        auth: {
          title: `User Login & Obtain JWT Token (${cName})`,
          desc: `Verify identity (username/email) & password and return a signed JWT authentication token along with user record.`,
          method: 'POST',
          badge: 'bg-teal-100 text-teal-800',
          path: `/api/${cName}/auth-with-password`,
          statusCode: '200 OK',
          js: `const res = await fetch('${baseUrl}/api/${cName}/auth-with-password', {\n  method: 'POST',\n  headers: { 'Content-Type': 'application/json' },\n  body: JSON.stringify({\n    identity: 'john@example.com',\n    password: 'secret123456'\n  })\n});\nconst { item } = await res.json();\nconsole.log(item.token, item.record);`,
          curl: `curl -X POST "${baseUrl}/api/${cName}/auth-with-password" \\\n  -H "Content-Type: application/json" \\\n  -d '{\n  "identity": "john@example.com",\n  "password": "secret123456"\n}'`
        },
        register: {
          title: `Register New Account (${cName})`,
          desc: `Create a new user account in ${cName} and immediately receive a signed JWT token + user record.`,
          method: 'POST',
          badge: 'bg-teal-100 text-teal-800',
          path: `/api/${cName}/register`,
          statusCode: '201 Created',
          js: `const res = await fetch('${baseUrl}/api/${cName}/register', {\n  method: 'POST',\n  headers: { 'Content-Type': 'application/json' },\n  body: JSON.stringify({\n    email: 'john@example.com',\n    username: 'john123',\n    name: 'John Doe',\n    password: 'secret123456',\n    passwordConfirm: 'secret123456'\n  })\n});\nconst { item } = await res.json();\nconsole.log(item.token, item.record);`,
          curl: `curl -X POST "${baseUrl}/api/${cName}/register" \\\n  -H "Content-Type: application/json" \\\n  -d '{\n  "email": "john@example.com",\n  "username": "john123",\n  "name": "John Doe",\n  "password": "secret123456",\n  "passwordConfirm": "secret123456"\n}'`
        },
        forgot: {
          title: `Forgot Password - Generate Reset Token (${cName})`,
          desc: `Generate a one-time password reset token for a user account by email or username.`,
          method: 'POST',
          badge: 'bg-teal-100 text-teal-800',
          path: `/api/${cName}/forgot-password`,
          statusCode: '200 OK',
          js: `const res = await fetch('${baseUrl}/api/${cName}/forgot-password', {\n  method: 'POST',\n  headers: { 'Content-Type': 'application/json' },\n  body: JSON.stringify({\n    email: 'john@example.com'\n  })\n});\nconst { item } = await res.json();\nconsole.log(item.email, item.reset_token);`,
          curl: `curl -X POST "${baseUrl}/api/${cName}/forgot-password" \\\n  -H "Content-Type: application/json" \\\n  -d '{\n  "email": "john@example.com"\n}'`
        },
        reset: {
          title: `Reset Password with Token (${cName})`,
          desc: `Verify the one-time reset token and set a new Argon2id-hashed password for the account.`,
          method: 'POST',
          badge: 'bg-teal-100 text-teal-800',
          path: `/api/${cName}/reset-password`,
          statusCode: '200 OK',
          js: `const res = await fetch('${baseUrl}/api/${cName}/reset-password', {\n  method: 'POST',\n  headers: { 'Content-Type': 'application/json' },\n  body: JSON.stringify({\n    token: 'd7f4a1c2e8b940bf7c53e01a8',\n    password: 'newSecretPassword123',\n    passwordConfirm: 'newSecretPassword123'\n  })\n});\nconst { item } = await res.json();\nconsole.log(item.message);`,
          curl: `curl -X POST "${baseUrl}/api/${cName}/reset-password" \\\n  -H "Content-Type: application/json" \\\n  -d '{\n  "token": "d7f4a1c2e8b940bf7c53e01a8",\n  "password": "newSecretPassword123",\n  "passwordConfirm": "newSecretPassword123"\n}'`
        },
        change_pw: {
          title: `Change Password for Logged-in User (${cName})`,
          desc: `Verify current password and update to a new password while returning a refreshed JWT token.`,
          method: 'POST',
          badge: 'bg-teal-100 text-teal-800',
          path: `/api/${cName}/change-password`,
          statusCode: '200 OK',
          js: `const res = await fetch('${baseUrl}/api/${cName}/change-password', {\n  method: 'POST',\n  headers: {\n    'Content-Type': 'application/json',\n    'Authorization': 'Bearer YOUR_JWT_TOKEN'\n  },\n  body: JSON.stringify({\n    oldPassword: 'secret123456',\n    newPassword: 'updatedPassword789',\n    passwordConfirm: 'updatedPassword789'\n  })\n});\nconst { item } = await res.json();\nconsole.log(item.token);`,
          curl: `curl -X POST "${baseUrl}/api/${cName}/change-password" \\\n  -H "Content-Type: application/json" \\\n  -H "Authorization: Bearer YOUR_JWT_TOKEN" \\\n  -d '{\n  "oldPassword": "secret123456",\n  "newPassword": "updatedPassword789",\n  "passwordConfirm": "updatedPassword789"\n}'`
        },
        me: {
          title: `Get Current User Profile & Refresh JWT (${cName})`,
          desc: `Fetch the authenticated user's record and receive a refreshed JWT token.`,
          method: 'GET',
          badge: 'bg-teal-100 text-teal-800',
          path: `/api/${cName}/me`,
          statusCode: '200 OK',
          js: `const res = await fetch('${baseUrl}/api/${cName}/me', {\n  headers: {\n    'Authorization': 'Bearer YOUR_JWT_TOKEN'\n  }\n});\nconst { item } = await res.json();\nconsole.log(item.record, item.token);`,
          curl: `curl -X GET "${baseUrl}/api/${cName}/me" \\\n  -H "Authorization: Bearer YOUR_JWT_TOKEN"`
        },
        transactions: {
          title: `Atomic Database Transaction / Batch (${cName})`,
          desc: `Execute multiple create, update, delete, or SQL operations inside a single atomic DB transaction (all-or-nothing rollback on failure).`,
          method: 'POST',
          badge: 'bg-amber-100 text-amber-800',
          path: `/api/transactions`,
          statusCode: '200 OK',
          js: `const res = await fetch('${baseUrl}/api/transactions', {\n  method: 'POST',\n  headers: {\n    'Content-Type': 'application/json',\n    'Authorization': 'Bearer YOUR_JWT_TOKEN'\n  },\n  body: JSON.stringify({\n    operations: [\n      {\n        action: 'create',\n        table: '${cName}',\n        data: ${JSON.stringify(sampleObj)}\n      },\n      {\n        action: 'update',\n        table: '${cName}',\n        id: 'rec_001',\n        data: ${JSON.stringify(sampleObj)}\n      }\n    ]\n  })\n});\nconst { item } = await res.json();`,
          curl: `curl -X POST "${baseUrl}/api/transactions" \\\n  -H "Content-Type: application/json" \\\n  -H "Authorization: Bearer YOUR_JWT_TOKEN" \\\n  -d '{\n  "operations": [\n    {\n      "action": "create",\n      "table": "${cName}",\n      "data": ${JSON.stringify(sampleObj)}\n    }\n  ]\n}'`
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
        <div class="space-y-4">
          <!-- 1. Request Code Snippet (JS Fetch / cURL) -->
          <div>
            <div class="flex items-center justify-between mb-1.5">
              <div class="flex items-center space-x-1.5 bg-slate-200/60 p-0.5 rounded-lg border border-slate-200">
                <button type="button" id="apiSnippetBtn_js" onclick="toggleApiSnippetMode('js')" class="px-2.5 py-0.5 rounded-md ${currentApiSnippetMode === 'js' ? 'bg-emerald-600 text-white font-bold shadow-2xs' : 'text-slate-600 hover:text-slate-900 font-semibold'} text-[10px] transition cursor-pointer">
                  JS Fetch
                </button>
                <button type="button" id="apiSnippetBtn_curl" onclick="toggleApiSnippetMode('curl')" class="px-2.5 py-0.5 rounded-md ${currentApiSnippetMode === 'curl' ? 'bg-emerald-600 text-white font-bold shadow-2xs' : 'text-slate-600 hover:text-slate-900 font-semibold'} text-[10px] transition cursor-pointer">
                  cURL
                </button>
              </div>
              <button type="button" onclick="copyToClipboard(currentApiSnippetMode === 'js' ? currentApiPreviewJs : currentApiPreviewCurl)" class="px-2.5 py-1 rounded-lg border border-slate-200 bg-white hover:bg-slate-50 text-slate-700 text-[10px] font-bold flex items-center space-x-1 transition cursor-pointer">
                <i class="fa-regular fa-copy text-[9px]"></i>
                <span>Copy Snippet</span>
              </button>
            </div>
            <pre id="apiSnippetPre_js" class="${currentApiSnippetMode === 'js' ? '' : 'hidden '}bg-slate-950 text-emerald-400 p-4 rounded-2xl font-mono text-[11px] overflow-x-auto max-h-[260px]">${d.js}</pre>
            <pre id="apiSnippetPre_curl" class="${currentApiSnippetMode === 'curl' ? '' : 'hidden '}bg-slate-950 text-emerald-400 p-4 rounded-2xl font-mono text-[11px] overflow-x-auto max-h-[260px]">${d.curl}</pre>
          </div>

          <!-- 2. JSON Response (Padi Framework) Below Code Snippet -->
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
            <pre class="bg-slate-950 text-teal-300 p-4 rounded-2xl font-mono text-[11px] overflow-x-auto max-h-[300px]">${currentApiPreviewResponseJson}</pre>
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
        if (settings.smtp) {
          const enabledInput = document.getElementById('settingSmtpEnabled');
          if (enabledInput) enabledInput.checked = Boolean(settings.smtp.enabled);
          const hostInput = document.getElementById('settingSmtpHost');
          if (hostInput) hostInput.value = settings.smtp.host || '';
          const portInput = document.getElementById('settingSmtpPort');
          if (portInput) portInput.value = settings.smtp.port || 587;
          const encInput = document.getElementById('settingSmtpEncryption');
          if (encInput) {
            let enc = (settings.smtp.encryption || 'opportunistic').toLowerCase();
            if (enc === 'tls' || enc === 'ssl') enc = 'always';
            if (enc === 'starttls') enc = 'opportunistic';
            if (enc === 'none') enc = 'disabled';
            encInput.value = enc;
          }
          const authMethodInput = document.getElementById('settingSmtpAuthMethod');
          if (authMethodInput) authMethodInput.value = settings.smtp.auth_method || 'LOGIN';
          const ehloInput = document.getElementById('settingSmtpEhloDomain');
          if (ehloInput) ehloInput.value = settings.smtp.ehlo_domain || '';
          const userInput = document.getElementById('settingSmtpUsername');
          if (userInput) userInput.value = settings.smtp.username || '';
          const passInput = document.getElementById('settingSmtpPassword');
          if (passInput) passInput.value = settings.smtp.password || '';
          const nameInput = document.getElementById('settingSmtpFromName');
          if (nameInput) nameInput.value = settings.smtp.from_name || '';
          const addrInput = document.getElementById('settingSmtpFromAddress');
          if (addrInput) addrInput.value = settings.smtp.from_address || '';

          const events = settings.smtp.events || {};
          const evWelcome = document.getElementById('settingEventWelcome');
          if (evWelcome) evWelcome.checked = events.welcome !== false;
          const evForgot = document.getElementById('settingEventForgotPassword');
          if (evForgot) evForgot.checked = events.forgot_password !== false;
          const evChanged = document.getElementById('settingEventPasswordChanged');
          if (evChanged) evChanged.checked = events.password_changed !== false;
          const evAdmin = document.getElementById('settingEventAdminCreated');
          if (evAdmin) evAdmin.checked = events.admin_created !== false;

          if (settings.smtp.templates) {
            ['welcome', 'forgot_password', 'password_changed', 'admin_created'].forEach(k => {
              if (settings.smtp.templates[k] && settings.smtp.templates[k].subject) {
                emailTemplates[k] = {
                  subject: settings.smtp.templates[k].subject,
                  body: settings.smtp.templates[k].body || ''
                };
              } else {
                emailTemplates[k] = { ...DEFAULT_EMAIL_TEMPLATES[k] };
              }
            });
          } else {
            ['welcome', 'forgot_password', 'password_changed', 'admin_created'].forEach(k => {
              emailTemplates[k] = { ...DEFAULT_EMAIL_TEMPLATES[k] };
            });
          }
          selectEmailTemplateTab(currentSelectedTemplateKey);
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

    function toggleSmtpMoreOptions() {
      const pane = document.getElementById('smtpMoreOptionsPane');
      const icon = document.getElementById('toggleMoreOptionsIcon');
      const text = document.getElementById('toggleMoreOptionsText');
      if (!pane) return;
      const isHidden = pane.classList.contains('hidden');
      if (isHidden) {
        pane.classList.remove('hidden');
        if (icon) {
          icon.classList.remove('fa-chevron-down');
          icon.classList.add('fa-chevron-up');
        }
        if (text) text.textContent = 'Hide more options';
      } else {
        pane.classList.add('hidden');
        if (icon) {
          icon.classList.remove('fa-chevron-up');
          icon.classList.add('fa-chevron-down');
        }
        if (text) text.textContent = 'Show more options';
      }
    }

    async function saveMailSettings() {
      const enabled = Boolean(document.getElementById('settingSmtpEnabled')?.checked);
      const host = document.getElementById('settingSmtpHost')?.value.trim() || '';
      const port = parseInt(document.getElementById('settingSmtpPort')?.value.trim() || '587', 10);
      const encryption = document.getElementById('settingSmtpEncryption')?.value || 'opportunistic';
      const auth_method = document.getElementById('settingSmtpAuthMethod')?.value || 'LOGIN';
      const ehlo_domain = document.getElementById('settingSmtpEhloDomain')?.value.trim() || '';
      const username = document.getElementById('settingSmtpUsername')?.value.trim() || '';
      const password = document.getElementById('settingSmtpPassword')?.value || '';
      const from_name = document.getElementById('settingSmtpFromName')?.value.trim() || '';
      const from_address = document.getElementById('settingSmtpFromAddress')?.value.trim() || '';

      const welcome = Boolean(document.getElementById('settingEventWelcome')?.checked);
      const forgot_password = Boolean(document.getElementById('settingEventForgotPassword')?.checked);
      const password_changed = Boolean(document.getElementById('settingEventPasswordChanged')?.checked);
      const admin_created = Boolean(document.getElementById('settingEventAdminCreated')?.checked);

      const headers = { 'Content-Type': 'application/json' };
      if (currentToken) {
        headers['Authorization'] = `Bearer ${currentToken}`;
      }

      try {
        const res = await fetch('/api/settings', {
          method: 'PATCH',
          headers,
          body: JSON.stringify({
            smtp: {
              enabled,
              host,
              port,
              encryption,
              auth_method,
              ehlo_domain,
              username,
              password,
              from_name,
              from_address,
              events: {
                welcome,
                forgot_password,
                password_changed,
                admin_created
              },
              templates: emailTemplates
            }
          })
        });
        if (res.ok) {
          showToast('Mail & SMTP settings & templates saved successfully!');
        } else {
          const d = await res.json();
          showToast(d.message || 'Failed to save mail settings', 'error');
        }
      } catch (err) {
        showToast('Network error while saving settings: ' + err.message, 'error');
      }
    }

    // ================= EMAIL TEMPLATES STATE & ACTIONS =================
    let currentSelectedTemplateKey = 'welcome';
    const DEFAULT_EMAIL_TEMPLATES = {
      welcome: {
        subject: "Welcome to {appName}!",
        body: `<h2 style="margin-top: 0; color: #0f172a; font-size: 22px; font-weight: 800;">Welcome, {name}! 🎉</h2>
<p>Thank you for creating an account with <strong>{appName}</strong>. Your account has been successfully set up and is ready to use.</p>

<div style="background-color: #f8fafc; border-radius: 12px; padding: 16px; margin: 20px 0; border: 1px solid #e2e8f0; font-size: 14px;">
  <div style="margin-bottom: 8px;"><strong>Email:</strong> <span style="color: #334155;">{email}</span></div>
  <div style="margin-bottom: 8px;"><strong>Collection / Table:</strong> <span style="font-family: monospace; background: #e2e8f0; padding: 2px 6px; border-radius: 4px;">{collection}</span></div>
  <div><strong>Status:</strong> <span style="color: #059669; font-weight: 600;">Active</span></div>
</div>

<p>You can now log in to your account and explore all available features:</p>
<div style="text-align: center;">
  <a href="{appUrl}" class="btn">Go to Dashboard &rarr;</a>
</div>

<p style="color: #64748b; font-size: 13px; margin-top: 30px;">If you did not register for this account, please contact your administrator or ignore this email.</p>`
      },
      forgot_password: {
        subject: "Password Reset Request - {appName}",
        body: `<h2 style="margin-top: 0; color: #0f172a; font-size: 22px; font-weight: 800;">Password Reset Request 🔐</h2>
<p>We received a request to reset the password for your account (<strong>{email}</strong>).</p>

<p>Use the following secure token to complete your password reset:</p>

<div class="token-box">
  {token}
</div>

<p style="font-size: 13px; color: #64748b;">Copy and paste this token into the password reset form in your application, or submit it to the <code>/reset-password</code> endpoint.</p>

<div style="text-align: center;">
  <a href="{appUrl}" class="btn">Open Application</a>
</div>

<div style="margin-top: 30px; padding: 14px; background-color: #fff1f2; border-left: 4px solid #f43f5e; border-radius: 8px; font-size: 13px; color: #9f1239;">
  <strong>Security note:</strong> If you did not request a password reset, you can safely ignore this email. Your current password will remain unchanged.
</div>`
      },
      password_changed: {
        subject: "Security Alert: Password Changed - {appName}",
        body: `<h2 style="margin-top: 0; color: #0f172a; font-size: 22px; font-weight: 800;">Password Successfully Changed 🛡️</h2>
<p>This is a confirmation that the password for your account (<strong>{email}</strong>) was changed on <strong>{date}</strong>.</p>

<div style="margin: 20px 0; padding: 16px; background-color: #f0fdf4; border-left: 4px solid #10b981; border-radius: 8px; font-size: 14px; color: #065f46;">
  Your account is now secured with the new password. All active sessions have been safely updated.
</div>

<div style="text-align: center;">
  <a href="{appUrl}" class="btn">Sign In to Your Account &rarr;</a>
</div>

<p style="color: #64748b; font-size: 13px; margin-top: 25px;">If you did not make this change, please contact your system administrator immediately to secure your account.</p>`
      },
      admin_created: {
        subject: "Administrator Access Granted - {appName}",
        body: `<h2 style="margin-top: 0; color: #0f172a; font-size: 22px; font-weight: 800;">Administrator Account Created 👑</h2>
<p>You have been granted administrator access to <strong>{appName}</strong> with the email address <strong>{email}</strong>.</p>

<p>You can now access the management console and configure databases, collections, and security settings:</p>

<div style="text-align: center;">
  <a href="{appUrl}" class="btn">Open Admin Console &rarr;</a>
</div>

<p style="color: #64748b; font-size: 13px; margin-top: 25px;">Please ensure your administrator credentials are stored securely and never shared.</p>`
      }
    };

    let emailTemplates = JSON.parse(JSON.stringify(DEFAULT_EMAIL_TEMPLATES));

    function selectEmailTemplateTab(key) {
      currentSelectedTemplateKey = key;
      const keys = ['welcome', 'forgot_password', 'password_changed', 'admin_created'];
      keys.forEach(k => {
        const btn = document.getElementById(`tmplTab_${k}`);
        if (!btn) return;
        const isSelected = k === key;
        btn.className = isSelected
          ? "px-3 py-1.5 rounded-lg text-xs font-bold bg-emerald-600 text-white shadow-2xs transition shrink-0 cursor-pointer"
          : "px-3 py-1.5 rounded-lg text-xs font-semibold text-slate-600 hover:bg-slate-100 transition shrink-0 cursor-pointer";
      });

      const tmpl = emailTemplates[key] || DEFAULT_EMAIL_TEMPLATES[key];
      const subjInput = document.getElementById('settingTmplSubject');
      const bodyInput = document.getElementById('settingTmplBody');
      if (subjInput) subjInput.value = tmpl.subject || '';
      if (bodyInput) bodyInput.value = tmpl.body || '';
    }

    function handleTemplateSubjectChange(e) {
      if (!emailTemplates[currentSelectedTemplateKey]) {
        emailTemplates[currentSelectedTemplateKey] = {};
      }
      emailTemplates[currentSelectedTemplateKey].subject = e.target.value;
    }

    function handleTemplateBodyChange(e) {
      if (!emailTemplates[currentSelectedTemplateKey]) {
        emailTemplates[currentSelectedTemplateKey] = {};
      }
      emailTemplates[currentSelectedTemplateKey].body = e.target.value;
    }

    function insertPlaceholderToTemplate(tag) {
      const textarea = document.getElementById('settingTmplBody');
      if (!textarea) return;
      const start = textarea.selectionStart;
      const end = textarea.selectionEnd;
      const val = textarea.value;
      textarea.value = val.substring(0, start) + tag + val.substring(end);
      textarea.selectionStart = textarea.selectionEnd = start + tag.length;
      textarea.focus();
      handleTemplateBodyChange({ target: textarea });
    }

    function resetCurrentEmailTemplateToDefault() {
      const def = DEFAULT_EMAIL_TEMPLATES[currentSelectedTemplateKey];
      if (!def) return;
      emailTemplates[currentSelectedTemplateKey] = { ...def };
      const subjInput = document.getElementById('settingTmplSubject');
      const bodyInput = document.getElementById('settingTmplBody');
      if (subjInput) subjInput.value = def.subject;
      if (bodyInput) bodyInput.value = def.body;
      showToast(`Template reset to system default`);
    }

    function previewCurrentEmailTemplate() {
      const modal = document.getElementById('emailPreviewModal');
      const frame = document.getElementById('emailPreviewFrame');
      const subjTitle = document.getElementById('emailPreviewSubjectTitle');
      if (!modal || !frame) return;

      const tmpl = emailTemplates[currentSelectedTemplateKey] || DEFAULT_EMAIL_TEMPLATES[currentSelectedTemplateKey];
      const appName = document.getElementById('settingAppName')?.value.trim() || 'PadiBase Studio';
      const appUrl = document.getElementById('settingAppUrl')?.value.trim() || window.location.origin;

      let renderedSubj = tmpl.subject || '';
      let renderedBody = tmpl.body || '';

      const sampleMap = {
        '{name}': 'Jane Doe',
        '{email}': 'jane.doe@example.com',
        '{appName}': appName,
        '{appUrl}': appUrl,
        '{token}': 'tkn_9a8b7c6d5e4f3a2b1c',
        '{collection}': 'users',
        '{date}': new Date().toLocaleString()
      };

      for (const [k, v] of Object.entries(sampleMap)) {
        renderedSubj = renderedSubj.replaceAll(k, v);
        renderedBody = renderedBody.replaceAll(k, v);
      }

      if (subjTitle) subjTitle.textContent = `Subject: ${renderedSubj}`;

      let fullHtml = renderedBody;
      if (!renderedBody.includes('<!DOCTYPE') && !renderedBody.includes('<html')) {
        fullHtml = `<!DOCTYPE html>
<html>
<head>
  <meta charset="UTF-8">
  <style>
    body { margin: 0; padding: 25px 15px; background-color: #f1f5f9; font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; color: #1e293b; }
    .card { max-width: 580px; margin: 0 auto; background-color: #ffffff; border-radius: 16px; overflow: hidden; border: 1px solid #e2e8f0; box-shadow: 0 4px 6px -1px rgba(0,0,0,0.05); }
    .header { background: linear-gradient(135deg, #0f172a 0%, #1e293b 100%); padding: 26px 32px; color: #ffffff; font-size: 18px; font-weight: 800; }
    .content { padding: 32px; line-height: 1.6; font-size: 15px; }
    .btn { display: inline-block; background-color: #059669; color: #ffffff !important; text-decoration: none; font-size: 14px; font-weight: 700; padding: 12px 24px; border-radius: 10px; margin: 18px 0; }
    .token-box { background-color: #f8fafc; border: 1px dashed #cbd5e1; border-radius: 10px; padding: 16px; font-family: monospace; font-size: 18px; font-weight: 700; text-align: center; color: #0f172a; margin: 18px 0; }
    .footer { padding: 20px 32px; background-color: #f8fafc; border-top: 1px solid #f1f5f9; text-align: center; font-size: 12px; color: #64748b; }
  </style>
</head>
<body>
  <div class="card">
    <div class="header">${appName}</div>
    <div class="content">${renderedBody}</div>
    <div class="footer">&copy; ${new Date().getFullYear()} ${appName}. All rights reserved.</div>
  </div>
</body>
</html>`;
      }

      frame.srcdoc = fullHtml;
      modal.classList.remove('hidden');
    }

    function closeEmailPreviewModal() {
      const modal = document.getElementById('emailPreviewModal');
      if (modal) modal.classList.add('hidden');
    }

    async function handleSendTestEmail() {
      const recipientInput = document.getElementById('testEmailRecipient');
      const email = recipientInput?.value.trim();
      const templateSelect = document.getElementById('testEmailTemplateSelect');
      const template = templateSelect ? templateSelect.value : 'test';
      const resultDiv = document.getElementById('testEmailResult');
      const btn = document.getElementById('sendTestEmailBtn');
      const icon = document.getElementById('sendTestEmailIcon');
      const text = document.getElementById('sendTestEmailText');

      if (!email || !email.includes('@')) {
        showToast('Please enter a valid recipient email address', 'error');
        if (recipientInput) recipientInput.focus();
        return;
      }

      if (btn) btn.disabled = true;
      if (icon) icon.className = 'fa-solid fa-spinner fa-spin text-xs';
      if (text) text.textContent = 'Sending...';
      if (resultDiv) resultDiv.classList.add('hidden');

      const headers = { 'Content-Type': 'application/json' };
      if (currentToken) {
        headers['Authorization'] = `Bearer ${currentToken}`;
      }

      try {
        const res = await fetch('/api/settings/test-email', {
          method: 'POST',
          headers,
          body: JSON.stringify({ email, template })
        });
        const data = await res.json();

        if (res.ok) {
          showToast(data.message || 'Test email sent successfully!');
          if (resultDiv) {
            resultDiv.className = 'p-3 rounded-lg text-xs font-semibold bg-emerald-50 text-emerald-800 border border-emerald-200';
            resultDiv.innerHTML = `<i class="fa-solid fa-circle-check text-emerald-600 mr-1.5"></i> ${data.message || 'Test email delivered successfully!'}`;
            resultDiv.classList.remove('hidden');
          }
        } else {
          showToast(data.message || 'Failed to send test email', 'error');
          if (resultDiv) {
            resultDiv.className = 'p-3 rounded-lg text-xs font-semibold bg-rose-50 text-rose-800 border border-rose-200';
            resultDiv.innerHTML = `<i class="fa-solid fa-triangle-exclamation text-rose-600 mr-1.5"></i> ${data.message || 'SMTP delivery failed'}`;
            resultDiv.classList.remove('hidden');
          }
        }
      } catch (err) {
        showToast('Error: ' + err.message, 'error');
        if (resultDiv) {
          resultDiv.className = 'p-3 rounded-lg text-xs font-semibold bg-rose-50 text-rose-800 border border-rose-200';
          resultDiv.innerHTML = `<i class="fa-solid fa-triangle-exclamation text-rose-600 mr-1.5"></i> Connection error: ${err.message}`;
          resultDiv.classList.remove('hidden');
        }
      } finally {
        if (btn) btn.disabled = false;
        if (icon) icon.className = 'fa-solid fa-vial text-xs';
        if (text) text.textContent = 'Send Test Email';
      }
    }

    function triggerUploadBackup() {
      const input = document.getElementById('backupUploadInput');
      if (input) {
        input.value = '';
        input.click();
      }
    }

    async function handleUploadBackupFile(event) {
      const file = event.target.files && event.target.files[0];
      if (!file) return;

      const fd = new FormData();
      fd.append('file', file);

      showToast(`Uploading backup file "${file.name}"...`);

      try {
        const res = await fetch('/api/backups/upload', {
          method: 'POST',
          body: fd
        });
        const data = await res.json();
        if (res.ok) {
          showToast(`Backup "${file.name}" uploaded successfully!`);
          loadBackups();
        } else {
          showToast(data.message || 'Failed to upload backup archive', 'error');
        }
      } catch (err) {
        showToast('Upload error: ' + err.message, 'error');
      }
    }

    async function loadBackups() {
      const tbody = document.getElementById('backupsTableBody');
      if (!tbody) return;
      try {
        const res = await fetch('/api/backups');
        const data = await res.json();
        const items = data.item || data.items || [];
        if (items.length === 0) {
          tbody.innerHTML = `<tr><td colspan="6" class="py-10 text-center text-slate-400 font-sans"><div class="flex flex-col items-center justify-center space-y-2"><i class="fa-solid fa-box-archive text-3xl text-slate-300"></i><p class="font-medium text-xs">No database backups found yet.</p><p class="text-[11px] text-slate-400">Click "Create Backup Now" to generate an archive containing your database and file uploads.</p></div></td></tr>`;
        } else {
          tbody.innerHTML = items.map(b => {
            const isZip = b.is_zip || b.key.endsWith('.zip');
            const typeBadge = isZip
              ? `<span class="px-2 py-0.5 rounded-md text-[10px] font-bold bg-emerald-100 text-emerald-800 border border-emerald-300 inline-flex items-center space-x-1"><i class="fa-solid fa-file-zipper text-[9px]"></i><span>ZIP (DB + Storage)</span></span>`
              : `<span class="px-2 py-0.5 rounded-md text-[10px] font-bold bg-slate-100 text-slate-600 inline-flex items-center space-x-1"><i class="fa-solid fa-database text-[9px]"></i><span>Database Snapshot</span></span>`;
            const filesCount = b.storage_files != null && b.storage_files > 0
              ? `<span class="px-2 py-0.5 rounded-md text-[10px] font-mono font-bold bg-teal-50 text-teal-700 border border-teal-200">${b.storage_files} file${b.storage_files === 1 ? '' : 's'}</span>`
              : (isZip ? `<span class="text-slate-400 text-[11px]">0 files</span>` : `<span class="text-slate-300 text-[11px]">-</span>`);

            return `
              <tr class="hover:bg-emerald-50/30 transition">
                <td class="py-3 px-4 font-bold text-slate-800">
                  <div class="flex items-center space-x-2">
                    <span class="w-6 h-6 rounded-lg bg-emerald-50 text-emerald-600 flex items-center justify-center text-xs shrink-0">
                      <i class="fa-solid fa-box-archive text-[10px]"></i>
                    </span>
                    <span class="truncate max-w-xs md:max-w-md font-mono text-xs">${b.key}</span>
                  </div>
                </td>
                <td class="py-3 px-4">${typeBadge}</td>
                <td class="py-3 px-4 text-slate-600 font-mono text-xs">${formatBytes(b.size)}</td>
                <td class="py-3 px-4">${filesCount}</td>
                <td class="py-3 px-4 text-slate-400 text-xs">${formatDateCell(b.created)}</td>
                <td class="py-3 px-4 text-right space-x-1.5 whitespace-nowrap">
                  <button type="button" onclick="downloadBackup('${b.key}')" class="px-2.5 py-1 rounded-lg text-emerald-700 hover:bg-emerald-50 font-bold transition inline-flex items-center space-x-1 text-xs cursor-pointer">
                    <i class="fa-solid fa-download text-[10px]"></i>
                    <span>Download</span>
                  </button>
                  <button onclick="handleRestoreBackup('${b.key}')" class="px-2.5 py-1 rounded-lg text-amber-700 hover:bg-amber-50 font-bold transition inline-flex items-center space-x-1 text-xs cursor-pointer">
                    <i class="fa-solid fa-rotate-left text-[10px]"></i>
                    <span>Restore</span>
                  </button>
                  <button onclick="handleDeleteBackup('${b.key}')" class="px-2.5 py-1 rounded-lg text-rose-600 hover:bg-rose-50 font-bold transition inline-flex items-center space-x-1 text-xs cursor-pointer">
                    <i class="fa-solid fa-trash-can text-[10px]"></i>
                    <span>Delete</span>
                  </button>
                </td>
              </tr>
            `;
          }).join('');
        }
      } catch (err) {
        console.error("Failed to load backups:", err);
      }
    }

    async function handleCreateBackup() {
      const btn = document.getElementById('createBackupBtn');
      const text = document.getElementById('createBackupBtnText');
      const icon = document.getElementById('createBackupIcon');
      if (btn) btn.disabled = true;
      if (text) text.textContent = 'Archiving DB & Storage...';
      if (icon) icon.className = 'fa-solid fa-spinner fa-spin text-xs';

      try {
        const res = await fetch('/api/backups', { method: 'POST' });
        const data = await res.json();
        if (res.ok) {
          const item = data.item || data;
          const filesCount = item.storage_files != null ? ` (${item.storage_files} upload files included)` : '';
          showToast(`Unified backup created successfully: ${item.key || 'archive.zip'}${filesCount}`);
          await loadBackups();
        } else {
          showToast(data.message || 'Failed to create backup archive', 'error');
        }
      } catch (err) {
        showToast('Backup error: ' + err.message, 'error');
      } finally {
        if (btn) btn.disabled = false;
        if (text) text.textContent = 'Create Backup Now';
        if (icon) icon.className = 'fa-solid fa-plus text-xs';
      }
    }

    // Alias for legacy templates
    window.createDatabaseBackup = handleCreateBackup;

    async function handleRestoreBackup(key) {
      const isConfirmed = confirm(
        `⚠️ RESTORE BACKUP WARNING!\n\n` +
        `Are you sure you want to restore the entire system from "${key}"?\n\n` +
        `• Current database data will be REPLACED by the backup snapshot.\n` +
        `• Stored physical files in "storage/" will be unpacked and updated.\n` +
        `• Active connections will be re-checkpointed.\n\n` +
        `This operation cannot be undone. Click OK to proceed.`
      );
      if (!isConfirmed) return;

      showToast(`Restoring system from "${key}"... please wait.`);

      try {
        const res = await fetch(`/api/backups/${encodeURIComponent(key)}/restore`, { method: 'POST' });
        const data = await res.json();
        if (res.ok) {
          showToast(data.message || 'System restored successfully!');
          await loadCollections();
          renderQuickMatrixTable();
        } else {
          showToast(data.message || 'Failed to restore backup', 'error');
        }
      } catch (err) {
        showToast('Restore error: ' + err.message, 'error');
      }
    }

    async function handleDeleteBackup(key) {
      if (!confirm(`Permanently delete backup file "${key}"?`)) return;
      try {
        const res = await fetch(`/api/backups/${encodeURIComponent(key)}`, { method: 'DELETE' });
        if (res.ok || res.status === 204) {
          showToast(`Backup "${key}" deleted`);
          loadBackups();
        } else {
          showToast('Failed to delete backup file', 'error');
        }
      } catch (err) {
        showToast('Delete error: ' + err.message, 'error');
      }
    }

    function downloadExportedSchema() {
      const blob = new Blob([JSON.stringify(collections, null, 2)], { type: 'application/json' });
      const a = document.createElement('a');
      a.href = URL.createObjectURL(blob);
      a.download = 'padibase_schema_blueprint.json';
      a.click();
      showToast('Schema blueprint downloaded as schema.json');
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
      a.href = `/api/${currentCollection}/export`;
      a.download = `${currentCollection}_export.json`;
      a.click();
    }

    function handleExportApiCollection(isFull = false) {
      const a = document.createElement('a');
      if (isFull) {
        a.href = `/api/api-collection?download=true`;
        a.download = `padi_base_api_collection.json`;
      } else {
        a.href = `/api/${currentCollection}/api-collection?download=true`;
        a.download = `${currentCollection}_api.json`;
      }
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      showToast(isFull ? 'Successfully downloaded full API collection (.json)' : `Successfully downloaded API endpoints for ${currentCollection} (.json)`);
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
        const res = await fetch(`/api/${currentCollection}/import`, {
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

    // ================= REALTIME SSE MONITOR =================
    let realtimeEvents = [];
    let isRealtimePaused = false;
    let pausedRealtimeQueue = [];
    let realtimeActionFilter = 'all';
    let realtimeTableFilterVal = 'all';
    let realtimeSearchText = '';
    let realtimeFormatPretty = true;
    let sseReconnectTimer = null;
    let sseRetryDelay = 2000;
    const MAX_REALTIME_EVENTS = 250;

    function escapeHtml(str) {
      if (str === null || str === undefined) return '';
      return String(str)
        .replace(/&/g, '&amp;')
        .replace(/</g, '&lt;')
        .replace(/>/g, '&gt;')
        .replace(/"/g, '&quot;')
        .replace(/'/g, '&#39;');
    }

    function setRealtimeStatus(status, message) {
      const badge = document.getElementById('realtimeStatusBadge');
      const dot = document.getElementById('realtimeStatusDot');
      const text = document.getElementById('realtimeStatusText');
      const railDot = document.getElementById('sseStatusDot');
      const reconnectBtn = document.getElementById('reconnectSseBtn');

      if (status === 'connected') {
        if (badge) badge.className = "inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-mono font-bold bg-emerald-50 text-emerald-700 border border-emerald-200";
        if (dot) dot.className = "w-2 h-2 rounded-full bg-emerald-500 animate-pulse mr-1.5";
        if (text) text.textContent = isRealtimePaused ? 'Paused' : 'Live';
        if (railDot) {
          railDot.className = "w-2 h-2 rounded-full bg-emerald-400 animate-pulse ml-auto hidden md:block";
          railDot.title = "Realtime SSE: Connected";
        }
        if (reconnectBtn) reconnectBtn.classList.add('hidden');
      } else if (status === 'connecting') {
        if (badge) badge.className = "inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-mono font-bold bg-amber-50 text-amber-700 border border-amber-200";
        if (dot) dot.className = "w-2 h-2 rounded-full bg-amber-500 animate-ping mr-1.5";
        if (text) text.textContent = message || 'Connecting...';
        if (railDot) {
          railDot.className = "w-2 h-2 rounded-full bg-amber-400 animate-ping ml-auto hidden md:block";
          railDot.title = "Realtime SSE: Connecting";
        }
        if (reconnectBtn) reconnectBtn.classList.remove('hidden');
      } else if (status === 'disconnected') {
        if (badge) badge.className = "inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-mono font-bold bg-rose-50 text-rose-700 border border-rose-200";
        if (dot) dot.className = "w-2 h-2 rounded-full bg-rose-500 mr-1.5";
        if (text) text.textContent = 'Disconnected';
        if (railDot) {
          railDot.className = "w-2 h-2 rounded-full bg-rose-500 ml-auto hidden md:block";
          railDot.title = "Realtime SSE: Disconnected";
        }
        if (reconnectBtn) reconnectBtn.classList.remove('hidden');
      }
    }

    function initSSE() {
      if (sseSource) {
        try { sseSource.close(); } catch (_) {}
        sseSource = null;
      }
      if (sseReconnectTimer) {
        clearTimeout(sseReconnectTimer);
        sseReconnectTimer = null;
      }

      const token = localStorage.getItem('padi_admin_token') || '';
      setRealtimeStatus('connecting', 'Connecting...');

      try {
        sseSource = new EventSource('/api/realtime?token=' + encodeURIComponent(token));

        sseSource.onopen = () => {
          sseRetryDelay = 2000;
          setRealtimeStatus('connected');
        };

        sseSource.onmessage = (event) => {
          try {
            if (!event.data || event.data === 'keep-alive-ping') return;
            const data = JSON.parse(event.data);
            handleIncomingRealtimeEvent(data);
            const targetTable = data.table || data.collection;
            if (targetTable === currentCollection) loadRecords();
          } catch (_) {}
        };

        sseSource.addEventListener('lagged', () => {
          showToast('Realtime stream resynchronized (slow consumer lagged)', 'warning');
        });

        sseSource.onerror = () => {
          setRealtimeStatus('connecting', 'Reconnecting...');
          if (sseSource) {
            try { sseSource.close(); } catch (_) {}
            sseSource = null;
          }
          if (!sseReconnectTimer) {
            sseReconnectTimer = setTimeout(() => {
              sseReconnectTimer = null;
              sseRetryDelay = Math.min(sseRetryDelay * 1.5, 15000);
              initSSE();
            }, sseRetryDelay);
          }
        };
      } catch (e) {
        setRealtimeStatus('disconnected');
      }
    }

    function reconnectSSEManually() {
      if (sseReconnectTimer) {
        clearTimeout(sseReconnectTimer);
        sseReconnectTimer = null;
      }
      sseRetryDelay = 2000;
      initSSE();
      showToast('Reconnecting to Realtime SSE stream...', 'info');
    }

    function handleIncomingRealtimeEvent(data) {
      const now = new Date();
      const evt = {
        id: 'evt_' + now.getTime() + '_' + Math.random().toString(36).substr(2, 6),
        action: String(data.action || 'event').toLowerCase(),
        table: String(data.table || data.collection || 'unknown'),
        record: data.record || {},
        recordId: (data.record && (data.record.id !== undefined && data.record.id !== null)) ? String(data.record.id) : '',
        timestamp: now.toTimeString().split(' ')[0] + '.' + String(now.getMilliseconds()).padStart(3, '0'),
        isoTime: now.toISOString()
      };

      if (isRealtimePaused) {
        pausedRealtimeQueue.push(evt);
        updatePauseButtonState();
        return;
      }

      realtimeEvents.unshift(evt);
      if (realtimeEvents.length > MAX_REALTIME_EVENTS) {
        realtimeEvents.length = MAX_REALTIME_EVENTS;
      }

      updateRealtimeTableOptions();
      updateRealtimeStats();
      renderRealtimeEvents();
    }

    function toggleRealtimePause() {
      isRealtimePaused = !isRealtimePaused;
      if (!isRealtimePaused && pausedRealtimeQueue.length > 0) {
        while (pausedRealtimeQueue.length > 0) {
          realtimeEvents.unshift(pausedRealtimeQueue.shift());
        }
        if (realtimeEvents.length > MAX_REALTIME_EVENTS) {
          realtimeEvents.length = MAX_REALTIME_EVENTS;
        }
        updateRealtimeTableOptions();
        updateRealtimeStats();
        renderRealtimeEvents();
      }
      updatePauseButtonState();
      setRealtimeStatus(sseSource && sseSource.readyState === EventSource.OPEN ? 'connected' : 'connecting');
    }

    function updatePauseButtonState() {
      const btn = document.getElementById('toggleRealtimePauseBtn');
      const icon = document.getElementById('realtimePauseIcon');
      const text = document.getElementById('realtimePauseText');
      if (!btn || !icon || !text) return;

      if (isRealtimePaused) {
        btn.className = "px-3 py-1.5 rounded-xl border border-amber-300 bg-amber-50 hover:bg-amber-100 text-xs font-bold text-amber-800 transition flex items-center space-x-1.5 shadow-2xs";
        icon.className = "fa-solid fa-play text-amber-700";
        const queued = pausedRealtimeQueue.length;
        text.textContent = queued > 0 ? `Resume (${queued})` : 'Resume';
      } else {
        btn.className = "px-3 py-1.5 rounded-xl border border-slate-200 bg-white hover:bg-slate-50 text-xs font-bold text-slate-700 transition flex items-center space-x-1.5 shadow-2xs";
        icon.className = "fa-solid fa-pause text-slate-500";
        text.textContent = 'Pause';
      }
    }

    function toggleRealtimeFormat() {
      realtimeFormatPretty = !realtimeFormatPretty;
      const lbl = document.getElementById('realtimeFormatLabel');
      if (lbl) lbl.textContent = realtimeFormatPretty ? 'Pretty JSON' : 'Compact JSON';
      renderRealtimeEvents();
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
      realtimeEvents = [];
      pausedRealtimeQueue = [];
      updateRealtimeStats();
      renderRealtimeEvents();
      updatePauseButtonState();
      showToast('Realtime logs cleared', 'info');
    }

    function exportRealtimeLogs() {
      if (realtimeEvents.length === 0) {
        showToast('No realtime events captured to export', 'warning');
        return;
      }
      const dataStr = "data:text/json;charset=utf-8," + encodeURIComponent(JSON.stringify(realtimeEvents, null, 2));
      const dlAnchor = document.createElement('a');
      dlAnchor.setAttribute("href", dataStr);
      dlAnchor.setAttribute("download", `padibase-realtime-logs-${new Date().toISOString().slice(0, 19).replace(/[:T]/g, '-')}.json`);
      document.body.appendChild(dlAnchor);
      dlAnchor.click();
      dlAnchor.remove();
      showToast(`Exported ${realtimeEvents.length} realtime events`, 'success');
    }

    function setRealtimeActionFilter(action) {
      realtimeActionFilter = action;
      document.querySelectorAll('[data-rt-action]').forEach(btn => {
        const act = btn.getAttribute('data-rt-action');
        if (act === action) {
          btn.className = "px-2.5 py-1 rounded-lg text-xs font-bold transition bg-slate-900 text-white shadow-2xs";
        } else {
          btn.className = "px-2 py-1 rounded-lg text-xs font-bold transition text-slate-600 hover:text-slate-900 hover:bg-slate-100";
        }
      });
      renderRealtimeEvents();
    }

    function applyRealtimeFilters() {
      const searchInput = document.getElementById('realtimeSearchInput');
      const tableSelect = document.getElementById('realtimeTableFilter');
      realtimeSearchText = (searchInput ? searchInput.value : '').trim().toLowerCase();
      realtimeTableFilterVal = tableSelect ? tableSelect.value : 'all';
      renderRealtimeEvents();
    }

    function updateRealtimeTableOptions() {
      const select = document.getElementById('realtimeTableFilter');
      if (!select) return;
      const currentVal = select.value || 'all';
      const tableNames = Array.from(new Set([
        ...(collections || []).map(c => c.name),
        ...realtimeEvents.map(e => e.table)
      ])).filter(Boolean).sort();

      let html = '<option value="all">All Tables</option>';
      tableNames.forEach(name => {
        html += `<option value="${escapeHtml(name)}">${escapeHtml(name)}</option>`;
      });
      select.innerHTML = html;
      if (tableNames.includes(currentVal)) {
        select.value = currentVal;
      } else {
        select.value = 'all';
      }
    }

    function updateRealtimeStats() {
      const statTotal = document.getElementById('rtStatTotal');
      const statCreate = document.getElementById('rtStatCreate');
      const statUpdate = document.getElementById('rtStatUpdate');
      const statDelete = document.getElementById('rtStatDelete');

      const total = realtimeEvents.length;
      let creates = 0, updates = 0, deletes = 0;
      for (let i = 0; i < total; i++) {
        const act = realtimeEvents[i].action;
        if (act === 'create') creates++;
        else if (act === 'update') updates++;
        else if (act === 'delete') deletes++;
      }

      if (statTotal) statTotal.textContent = total;
      if (statCreate) statCreate.textContent = creates;
      if (statUpdate) statUpdate.textContent = updates;
      if (statDelete) statDelete.textContent = deletes;
    }

    function openRecordFromRealtime(table, recordId) {
      if (!table) return;
      switchMainView('collections');
      selectCollection(table);
      if (recordId !== undefined && recordId !== null && recordId !== '') {
        const filterInput = document.getElementById('recordFilterInput');
        if (filterInput) {
          filterInput.value = `id = '${recordId}'`;
          applyFilterSearch();
        }
      }
    }

    function renderRealtimeEvents() {
      const container = document.getElementById('eventLogContainer');
      if (!container) return;

      if (realtimeEvents.length === 0) {
        container.innerHTML = `
          <div class="h-full min-h-[300px] flex flex-col items-center justify-center text-center p-8 text-slate-400 space-y-3">
            <div class="w-14 h-14 rounded-2xl bg-slate-900 border border-slate-800 flex items-center justify-center text-teal-400 shadow-lg">
              <i class="fa-solid fa-wave-square text-2xl animate-pulse"></i>
            </div>
            <div class="space-y-1 max-w-md">
              <h3 class="text-sm font-bold text-slate-200">Realtime Monitor Active</h3>
              <p class="text-xs text-slate-400 leading-relaxed font-sans">
                Listening for live <code class="text-emerald-400 font-bold">create</code>, <code class="text-amber-400 font-bold">update</code>, and <code class="text-rose-400 font-bold">delete</code> mutations streamed via SSE.
              </p>
              <p class="text-[11px] text-slate-500 font-sans mt-2">
                Make any changes in the Tables view or via REST API to watch live events stream here in real-time.
              </p>
            </div>
          </div>
        `;
        return;
      }

      // Filter events
      const filtered = realtimeEvents.filter(evt => {
        if (realtimeActionFilter !== 'all' && evt.action !== realtimeActionFilter) return false;
        if (realtimeTableFilterVal !== 'all' && evt.table !== realtimeTableFilterVal) return false;
        if (realtimeSearchText) {
          const matchId = evt.recordId && evt.recordId.toLowerCase().includes(realtimeSearchText);
          const matchTable = evt.table && evt.table.toLowerCase().includes(realtimeSearchText);
          const matchPayload = JSON.stringify(evt.record).toLowerCase().includes(realtimeSearchText);
          if (!matchId && !matchTable && !matchPayload) return false;
        }
        return true;
      });

      if (filtered.length === 0) {
        container.innerHTML = `
          <div class="h-64 flex flex-col items-center justify-center text-center p-8 text-slate-500 space-y-2">
            <i class="fa-solid fa-filter-circle-xmark text-2xl text-slate-600 mb-1"></i>
            <div class="text-xs font-bold text-slate-400">No events match your current filter</div>
            <button onclick="resetRealtimeFilters()" class="text-xs font-semibold text-emerald-400 hover:text-emerald-300 underline mt-1">
              Reset Filters
            </button>
          </div>
        `;
        return;
      }

      container.innerHTML = filtered.map(evt => {
        let badgeCls = 'text-emerald-400 bg-emerald-950/80 border-emerald-800/70';
        if (evt.action === 'update') badgeCls = 'text-amber-400 bg-amber-950/80 border-amber-800/70';
        if (evt.action === 'delete') badgeCls = 'text-rose-400 bg-rose-950/80 border-rose-800/70';

        const jsonStr = realtimeFormatPretty ? JSON.stringify(evt.record, null, 2) : JSON.stringify(evt.record);
        const highlighted = syntaxHighlightJson(jsonStr);
        const encodedRecord = encodeURIComponent(JSON.stringify(evt.record || {}));

        return `
          <div class="border border-slate-800/90 bg-slate-900/75 hover:bg-slate-900/95 rounded-xl p-3 sm:p-4 space-y-2.5 hover:border-slate-700 transition shadow-xs">
            <div class="flex items-center justify-between gap-2 flex-wrap">
              <div class="flex items-center space-x-2 shrink-0 flex-wrap gap-y-1">
                <span class="text-slate-500 font-mono text-xs whitespace-nowrap shrink-0">[${evt.timestamp}]</span>
                <span class="font-extrabold uppercase px-2.5 py-0.5 rounded-md text-[11px] border ${badgeCls} shrink-0">${evt.action.toUpperCase()}</span>
                <button type="button" onclick="openRecordFromRealtime('${escapeHtml(evt.table)}', '')" class="text-teal-300 hover:text-teal-200 bg-teal-950/60 hover:bg-teal-900/60 border border-teal-800/60 px-2 py-0.5 rounded-md font-bold text-xs font-mono shrink-0 transition flex items-center space-x-1" title="Jump to table '${escapeHtml(evt.table)}'">
                  <i class="fa-solid fa-table-cells text-[10px]"></i>
                  <span>${escapeHtml(evt.table)}</span>
                </button>
                ${evt.recordId ? `
                  <span class="text-slate-300 text-xs font-mono bg-slate-800/90 border border-slate-700/70 px-2 py-0.5 rounded-md shrink-0 flex items-center space-x-1">
                    <span class="text-slate-500">#</span><span>${escapeHtml(evt.recordId)}</span>
                  </span>
                ` : ''}
              </div>
              <div class="flex items-center space-x-2 shrink-0">
                ${evt.recordId ? `
                  <button type="button" onclick="openRecordFromRealtime('${escapeHtml(evt.table)}', '${escapeHtml(evt.recordId)}')" class="px-2.5 py-1 rounded-lg bg-slate-800 hover:bg-emerald-950/80 hover:text-emerald-300 text-slate-300 text-xs font-bold transition flex items-center space-x-1.5 border border-slate-700/80 hover:border-emerald-700/60" title="Open record #${escapeHtml(evt.recordId)} in table">
                    <i class="fa-solid fa-arrow-up-right-from-square text-[11px]"></i>
                    <span>View in Table</span>
                  </button>
                ` : ''}
                <button type="button" onclick="copyRealtimePayload(this)" class="px-2.5 py-1 rounded-lg bg-slate-800 hover:bg-slate-700 text-slate-300 hover:text-white text-xs font-bold transition flex items-center space-x-1.5 border border-slate-700/80">
                  <i class="fa-regular fa-copy text-[11px]"></i>
                  <span>Copy JSON</span>
                </button>
              </div>
            </div>
            <div class="bg-slate-950/95 rounded-lg p-3 border border-slate-900/90 overflow-x-auto">
              <pre class="text-slate-200 text-xs font-mono leading-relaxed whitespace-pre-wrap break-all select-all" data-realtime-record="${encodedRecord}">${highlighted}</pre>
            </div>
          </div>
        `;
      }).join('');
    }

    function resetRealtimeFilters() {
      realtimeActionFilter = 'all';
      realtimeTableFilterVal = 'all';
      realtimeSearchText = '';
      const searchInput = document.getElementById('realtimeSearchInput');
      if (searchInput) searchInput.value = '';
      const tableSelect = document.getElementById('realtimeTableFilter');
      if (tableSelect) tableSelect.value = 'all';
      setRealtimeActionFilter('all');
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

    // ================= CRON & BACKGROUND SCHEDULED TASKS =================
    let cronJobsList = [];
    let currentCronJobType = 'sql';

    function describeCronSchedule(schedule) {
      if (!schedule) return '';
      const s = schedule.trim().toLowerCase();
      if (s === '@yearly' || s === '@annually') return 'Once a year (Jan 1, 00:00)';
      if (s === '@monthly') return 'Once a month (1st, 00:00)';
      if (s === '@weekly') return 'Once a week (Sunday, 00:00)';
      if (s === '@daily' || s === '@midnight' || s === '0 0 * * *') return 'Every day at 00:00';
      if (s === '@hourly' || s === '0 * * * *') return 'Every hour at :00';
      if (s === '*/5 * * * *' || s === '@every 5m') return 'Every 5 minutes';
      if (s === '*/10 * * * *' || s === '@every 10m') return 'Every 10 minutes';
      if (s === '*/15 * * * *' || s === '@every 15m') return 'Every 15 minutes';
      if (s === '*/30 * * * *' || s === '@every 30m') return 'Every 30 minutes';
      if (s === '0 */2 * * *' || s === '@every 2h') return 'Every 2 hours';
      if (s === '0 */6 * * *' || s === '@every 6h') return 'Every 6 hours';
      if (s === '0 */12 * * *' || s === '@every 12h') return 'Every 12 hours';
      return 'Custom interval';
    }

    async function loadCronJobs() {
      const loading = document.getElementById('cronLoadingState');
      const empty = document.getElementById('cronEmptyState');
      const tableWrapper = document.getElementById('cronTableWrapper');
      const statTotal = document.getElementById('cronStatTotal');
      const statActive = document.getElementById('cronStatActive');

      if (loading) loading.classList.remove('hidden');
      if (empty) empty.classList.add('hidden');
      if (tableWrapper) tableWrapper.classList.add('hidden');

      const headers = { 'Content-Type': 'application/json' };
      if (currentToken) headers['Authorization'] = `Bearer ${currentToken}`;

      try {
        const res = await fetch('/api/cron/jobs', { headers });
        if (loading) loading.classList.add('hidden');

        if (!res.ok) {
          const errData = await res.json().catch(() => ({}));
          showToast(errData.message || 'Failed to load cron jobs', 'error');
          return;
        }

        const data = await res.json();
        cronJobsList = data.items || [];

        const total = cronJobsList.length;
        const active = cronJobsList.filter(j => j.enabled).length;

        if (statTotal) statTotal.textContent = total;
        if (statActive) statActive.textContent = active;

        if (total === 0) {
          if (empty) empty.classList.remove('hidden');
          if (tableWrapper) tableWrapper.classList.add('hidden');
        } else {
          if (empty) empty.classList.add('hidden');
          if (tableWrapper) tableWrapper.classList.remove('hidden');
          renderCronJobsTable();
        }
      } catch (err) {
        if (loading) loading.classList.add('hidden');
        showToast('Error loading cron jobs: ' + err.message, 'error');
      }
    }

    function renderCronJobsTable() {
      const tbody = document.getElementById('cronTableBody');
      if (!tbody) return;

      tbody.innerHTML = cronJobsList.map(item => {
        const job = item.job || item;
        const nextRun = item.next_run_at;
        const isSql = (job.job_type || 'sql').toLowerCase() === 'sql';
        const typeBadge = isSql
          ? '<span class="px-2 py-0.5 rounded-md font-mono text-[10px] font-bold bg-purple-100 text-purple-700 border border-purple-200"><i class="fa-solid fa-database text-[9px] mr-1"></i>SQL</span>'
          : `<span class="px-2 py-0.5 rounded-md font-mono text-[10px] font-bold bg-blue-100 text-blue-700 border border-blue-200"><i class="fa-solid fa-globe text-[9px] mr-1"></i>HTTP ${job.http_method || 'GET'}</span>`;

        const scheduleDesc = describeCronSchedule(job.schedule);
        const lastStatus = job.last_status || 'none';
        let statusBadge = '<span class="text-slate-400 font-mono text-[11px]">Never run</span>';

        if (lastStatus === 'success') {
          statusBadge = `<span class="inline-flex items-center px-2 py-0.5 rounded-md font-mono text-[10px] font-bold bg-emerald-50 text-emerald-700 border border-emerald-200"><i class="fa-solid fa-circle-check text-[9px] mr-1"></i>success (${job.last_duration_ms || 0}ms)</span>`;
        } else if (lastStatus === 'failed') {
          statusBadge = `<span class="inline-flex items-center px-2 py-0.5 rounded-md font-mono text-[10px] font-bold bg-rose-50 text-rose-700 border border-rose-200" title="${escapeHtml(job.last_error || '')}"><i class="fa-solid fa-triangle-exclamation text-[9px] mr-1"></i>failed (${job.last_duration_ms || 0}ms)</span>`;
        }

        const detailSnippet = isSql
          ? (job.sql_query || '').substring(0, 70) + ((job.sql_query || '').length > 70 ? '...' : '')
          : (job.http_url || '');

        return `
          <tr class="hover:bg-slate-50/80 transition group">
            <!-- Active Toggle -->
            <td class="p-3 text-center align-middle">
              <label class="relative inline-flex items-center cursor-pointer">
                <input type="checkbox" onchange="toggleCronJobEnabled('${job.id}', this.checked)" ${job.enabled ? 'checked' : ''} class="sr-only peer">
                <div class="w-8 h-4.5 bg-slate-200 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-slate-300 after:border after:rounded-full after:h-3.5 after:w-3.5 after:transition-all peer-checked:bg-emerald-600"></div>
              </label>
            </td>

            <!-- Name & Detail -->
            <td class="p-3">
              <div class="flex items-center space-x-2">
                <span class="font-bold text-slate-800 text-xs">${escapeHtml(job.name)}</span>
                ${typeBadge}
              </div>
              <div class="text-[11px] font-mono text-slate-400 truncate max-w-sm sm:max-w-md mt-0.5" title="${escapeHtml(detailSnippet)}">
                ${escapeHtml(detailSnippet)}
              </div>
            </td>

            <!-- Schedule -->
            <td class="p-3">
              <div class="font-mono text-xs font-bold text-slate-700">${escapeHtml(job.schedule)}</div>
              <div class="text-[10px] text-slate-400">${escapeHtml(scheduleDesc)}</div>
            </td>

            <!-- Next Run -->
            <td class="p-3 font-mono text-[11px] text-slate-600">
              ${job.enabled && nextRun ? formatDateTime(nextRun) : '<span class="text-slate-400 italic">Paused</span>'}
            </td>

            <!-- Last Run & Status -->
            <td class="p-3">
              <div>${statusBadge}</div>
              <div class="text-[10px] text-slate-400 font-mono mt-0.5">${job.last_run_at ? formatDateTime(job.last_run_at) : '--'}</div>
            </td>

            <!-- Actions -->
            <td class="p-3 text-right space-x-1 whitespace-nowrap">
              <button type="button" onclick="runCronJobNow('${job.id}')" id="cronRunBtn_${job.id}" class="p-1.5 rounded-lg border border-slate-200 hover:border-emerald-300 hover:bg-emerald-50 text-slate-600 hover:text-emerald-700 transition cursor-pointer" title="Trigger manual execution now">
                <i class="fa-solid fa-play text-xs"></i>
              </button>
              <button type="button" onclick="viewCronJobResult('${job.id}')" class="p-1.5 rounded-lg border border-slate-200 hover:border-blue-300 hover:bg-blue-50 text-slate-600 hover:text-blue-700 transition cursor-pointer" title="View execution log & output">
                <i class="fa-solid fa-file-lines text-xs"></i>
              </button>
              <button type="button" onclick="openCronJobModal('${job.id}')" class="p-1.5 rounded-lg border border-slate-200 hover:border-indigo-300 hover:bg-indigo-50 text-slate-600 hover:text-indigo-700 transition cursor-pointer" title="Edit task">
                <i class="fa-solid fa-pencil text-xs"></i>
              </button>
              <button type="button" onclick="deleteCronJob('${job.id}')" class="p-1.5 rounded-lg border border-slate-200 hover:border-rose-300 hover:bg-rose-50 text-slate-400 hover:text-rose-600 transition cursor-pointer" title="Delete task">
                <i class="fa-solid fa-trash-can text-xs"></i>
              </button>
            </td>
          </tr>
        `;
      }).join('');
    }

    function openCronJobModal(id = null) {
      const modal = document.getElementById('cronJobModal');
      const title = document.getElementById('cronModalTitle');
      const editId = document.getElementById('cronJobEditId');
      const nameInput = document.getElementById('cronJobNameInput');
      const scheduleInput = document.getElementById('cronJobScheduleInput');
      const enabledInput = document.getElementById('cronJobEnabledInput');
      const sqlInput = document.getElementById('cronSqlInput');
      const httpMethodInput = document.getElementById('cronHttpMethodInput');
      const httpUrlInput = document.getElementById('cronHttpUrlInput');
      const httpHeadersInput = document.getElementById('cronHttpHeadersInput');
      const httpBodyInput = document.getElementById('cronHttpBodyInput');

      if (!modal) return;

      if (id) {
        const item = cronJobsList.find(j => (j.job ? j.job.id : j.id) === id);
        const job = item ? (item.job || item) : null;
        if (!job) return;

        if (title) title.textContent = 'Edit Scheduled Task';
        if (editId) editId.value = job.id;
        if (nameInput) nameInput.value = job.name || '';
        if (scheduleInput) scheduleInput.value = job.schedule || '';
        if (enabledInput) enabledInput.checked = job.enabled !== false;

        switchCronJobType(job.job_type || 'sql');

        if (sqlInput) sqlInput.value = job.sql_query || '';
        if (httpMethodInput) httpMethodInput.value = job.http_method || 'GET';
        if (httpUrlInput) httpUrlInput.value = job.http_url || '';
        if (httpHeadersInput) {
          httpHeadersInput.value = job.http_headers ? JSON.stringify(job.http_headers, null, 2) : '';
        }
        if (httpBodyInput) httpBodyInput.value = job.http_body || '';
      } else {
        if (title) title.textContent = 'Add Scheduled Task';
        if (editId) editId.value = '';
        if (nameInput) nameInput.value = '';
        if (scheduleInput) scheduleInput.value = '*/15 * * * *';
        if (enabledInput) enabledInput.checked = true;

        switchCronJobType('sql');

        if (sqlInput) sqlInput.value = '';
        if (httpMethodInput) httpMethodInput.value = 'GET';
        if (httpUrlInput) httpUrlInput.value = '';
        if (httpHeadersInput) httpHeadersInput.value = '';
        if (httpBodyInput) httpBodyInput.value = '';
      }

      updateCronSchedulePreview();
      modal.classList.remove('hidden');
    }

    function closeCronJobModal() {
      const modal = document.getElementById('cronJobModal');
      if (modal) modal.classList.add('hidden');
    }

    function switchCronJobType(type) {
      currentCronJobType = type;
      const btnSql = document.getElementById('cronTypeBtn_sql');
      const btnHttp = document.getElementById('cronTypeBtn_http');
      const paneSql = document.getElementById('cronPane_sql');
      const paneHttp = document.getElementById('cronPane_http');

      if (type === 'sql') {
        if (btnSql) {
          btnSql.className = "py-2 rounded-lg text-xs font-bold transition flex items-center justify-center space-x-1.5 bg-white text-purple-700 shadow-2xs";
        }
        if (btnHttp) {
          btnHttp.className = "py-2 rounded-lg text-xs font-semibold text-slate-600 hover:text-slate-900 transition flex items-center justify-center space-x-1.5";
        }
        if (paneSql) paneSql.classList.remove('hidden');
        if (paneHttp) paneHttp.classList.add('hidden');
      } else {
        if (btnSql) {
          btnSql.className = "py-2 rounded-lg text-xs font-semibold text-slate-600 hover:text-slate-900 transition flex items-center justify-center space-x-1.5";
        }
        if (btnHttp) {
          btnHttp.className = "py-2 rounded-lg text-xs font-bold transition flex items-center justify-center space-x-1.5 bg-white text-blue-700 shadow-2xs";
        }
        if (paneSql) paneSql.classList.add('hidden');
        if (paneHttp) paneHttp.classList.remove('hidden');
      }
    }

    function setCronSchedulePreset(expr) {
      const input = document.getElementById('cronJobScheduleInput');
      if (input) {
        input.value = expr;
        updateCronSchedulePreview();
      }
    }

    function updateCronSchedulePreview() {
      const input = document.getElementById('cronJobScheduleInput');
      const preview = document.getElementById('cronSchedulePreview');
      if (!input || !preview) return;
      const val = input.value.trim();
      preview.textContent = describeCronSchedule(val);
    }

    async function handleSaveCronJob(event) {
      event.preventDefault();
      const editId = document.getElementById('cronJobEditId')?.value;
      const name = document.getElementById('cronJobNameInput')?.value.trim();
      const schedule = document.getElementById('cronJobScheduleInput')?.value.trim();
      const enabled = Boolean(document.getElementById('cronJobEnabledInput')?.checked);
      const sql_query = document.getElementById('cronSqlInput')?.value.trim();
      const http_method = document.getElementById('cronHttpMethodInput')?.value;
      const http_url = document.getElementById('cronHttpUrlInput')?.value.trim();
      const rawHeaders = document.getElementById('cronHttpHeadersInput')?.value.trim();
      const http_body = document.getElementById('cronHttpBodyInput')?.value;

      if (!name) {
        showToast('Please enter a task name', 'error');
        return;
      }
      if (!schedule) {
        showToast('Please enter a cron schedule expression', 'error');
        return;
      }

      let http_headers = null;
      if (rawHeaders) {
        try {
          http_headers = JSON.parse(rawHeaders);
        } catch (_) {
          showToast('HTTP Headers must be a valid JSON object', 'error');
          return;
        }
      }

      if (currentCronJobType === 'sql' && !sql_query) {
        showToast('Please provide at least one SQL statement to execute', 'error');
        return;
      }

      if (currentCronJobType === 'http' && !http_url) {
        showToast('Please enter an endpoint URL for HTTP task', 'error');
        return;
      }

      const payload = {
        name,
        schedule,
        job_type: currentCronJobType,
        enabled,
        sql_query: currentCronJobType === 'sql' ? sql_query : null,
        http_method: currentCronJobType === 'http' ? http_method : null,
        http_url: currentCronJobType === 'http' ? http_url : null,
        http_headers: currentCronJobType === 'http' ? http_headers : null,
        http_body: currentCronJobType === 'http' ? http_body : null
      };

      const headers = { 'Content-Type': 'application/json' };
      if (currentToken) headers['Authorization'] = `Bearer ${currentToken}`;

      const btn = document.getElementById('saveCronJobBtn');
      if (btn) btn.disabled = true;

      try {
        const url = editId ? `/api/cron/jobs/${editId}` : '/api/cron/jobs';
        const method = editId ? 'PATCH' : 'POST';

        const res = await fetch(url, {
          method,
          headers,
          body: JSON.stringify(payload)
        });

        if (btn) btn.disabled = false;

        if (res.ok) {
          showToast(`Cron task "${name}" saved successfully!`);
          closeCronJobModal();
          await loadCronJobs();
        } else {
          const errData = await res.json().catch(() => ({}));
          showToast(errData.message || 'Failed to save task', 'error');
        }
      } catch (err) {
        if (btn) btn.disabled = false;
        showToast('Network error: ' + err.message, 'error');
      }
    }

    async function runCronJobNow(id) {
      const btn = document.getElementById(`cronRunBtn_${id}`);
      let origHtml = '';
      if (btn) {
        origHtml = btn.innerHTML;
        btn.innerHTML = '<i class="fa-solid fa-circle-notch fa-spin text-xs text-emerald-600"></i>';
        btn.disabled = true;
      }

      const headers = { 'Content-Type': 'application/json' };
      if (currentToken) headers['Authorization'] = `Bearer ${currentToken}`;

      try {
        const res = await fetch(`/api/cron/jobs/${id}/run`, {
          method: 'POST',
          headers
        });
        const data = await res.json();

        if (btn) {
          btn.innerHTML = origHtml;
          btn.disabled = false;
        }

        if (res.ok) {
          const dur = data.result ? data.result.duration_ms : 0;
          showToast(`Cron job executed successfully in ${dur}ms!`);
        } else {
          showToast(data.message || (data.result && data.result.error) || 'Job execution failed', 'error');
        }

        await loadCronJobs();
      } catch (err) {
        if (btn) {
          btn.innerHTML = origHtml;
          btn.disabled = false;
        }
        showToast('Execution trigger failed: ' + err.message, 'error');
      }
    }

    async function toggleCronJobEnabled(id, enabled) {
      const headers = { 'Content-Type': 'application/json' };
      if (currentToken) headers['Authorization'] = `Bearer ${currentToken}`;

      try {
        const res = await fetch(`/api/cron/jobs/${id}`, {
          method: 'PATCH',
          headers,
          body: JSON.stringify({ enabled })
        });
        if (res.ok) {
          showToast(`Task ${enabled ? 'enabled' : 'disabled'}`);
          await loadCronJobs();
        } else {
          showToast('Failed to toggle task status', 'error');
          await loadCronJobs();
        }
      } catch (err) {
        showToast('Network error: ' + err.message, 'error');
      }
    }

    async function deleteCronJob(id) {
      const item = cronJobsList.find(j => (j.job ? j.job.id : j.id) === id);
      const name = item ? (item.job ? item.job.name : item.name) : 'this task';

      if (!confirm(`Are you sure you want to delete the scheduled task "${name}"?`)) {
        return;
      }

      const headers = { 'Content-Type': 'application/json' };
      if (currentToken) headers['Authorization'] = `Bearer ${currentToken}`;

      try {
        const res = await fetch(`/api/cron/jobs/${id}`, {
          method: 'DELETE',
          headers
        });
        if (res.ok) {
          showToast(`Task "${name}" deleted successfully`);
          await loadCronJobs();
        } else {
          showToast('Failed to delete task', 'error');
        }
      } catch (err) {
        showToast('Network error: ' + err.message, 'error');
      }
    }

    function viewCronJobResult(id) {
      const item = cronJobsList.find(j => (j.job ? j.job.id : j.id) === id);
      const job = item ? (item.job || item) : null;
      if (!job) return;

      const modal = document.getElementById('cronResultModal');
      const nameEl = document.getElementById('cronResultTaskName');
      const statusEl = document.getElementById('cronResultStatus');
      const durationEl = document.getElementById('cronResultDuration');
      const timeEl = document.getElementById('cronResultTime');
      const outputEl = document.getElementById('cronResultOutput');
      const errContainer = document.getElementById('cronResultErrorContainer');
      const errEl = document.getElementById('cronResultError');

      if (!modal) return;

      if (nameEl) nameEl.textContent = job.name;
      if (statusEl) {
        const isOk = job.last_status === 'success';
        statusEl.textContent = (job.last_status || 'Never executed').toUpperCase();
        statusEl.className = isOk
          ? 'font-bold font-mono mt-0.5 text-emerald-600'
          : (job.last_status === 'failed' ? 'font-bold font-mono mt-0.5 text-rose-600' : 'font-bold font-mono mt-0.5 text-slate-400');
      }
      if (durationEl) durationEl.textContent = job.last_duration_ms != null ? `${job.last_duration_ms} ms` : '--';
      if (timeEl) timeEl.textContent = job.last_run_at ? formatDateTime(job.last_run_at) : '--';

      if (outputEl) {
        outputEl.textContent = job.last_result || (job.last_status ? '(No textual output)' : 'Task has not executed yet.');
      }

      if (job.last_error) {
        if (errContainer) errContainer.classList.remove('hidden');
        if (errEl) errEl.textContent = job.last_error;
      } else {
        if (errContainer) errContainer.classList.add('hidden');
      }

      modal.classList.remove('hidden');
    }

    function closeCronResultModal() {
      const modal = document.getElementById('cronResultModal');
      if (modal) modal.classList.add('hidden');
    }

    function createPresetCronJob(preset) {
      openCronJobModal();
      if (preset === 'vacuum') {
        const nameInput = document.getElementById('cronJobNameInput');
        const scheduleInput = document.getElementById('cronJobScheduleInput');
        const sqlInput = document.getElementById('cronSqlInput');
        if (nameInput) nameInput.value = 'Daily Database Vacuum & Optimize';
        if (scheduleInput) scheduleInput.value = '0 3 * * *'; // 03:00 AM
        switchCronJobType('sql');
        if (sqlInput) sqlInput.value = 'VACUUM; ANALYZE;';
      } else if (preset === 'health') {
        const nameInput = document.getElementById('cronJobNameInput');
        const scheduleInput = document.getElementById('cronJobScheduleInput');
        const httpUrlInput = document.getElementById('cronHttpUrlInput');
        if (nameInput) nameInput.value = 'Hourly Health Check Ping';
        if (scheduleInput) scheduleInput.value = '0 * * * *'; // Hourly
        switchCronJobType('http');
        if (httpUrlInput) httpUrlInput.value = '/api/health';
      }
      updateCronSchedulePreview();
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
        closeCronJobModal();
        closeCronResultModal();
      }
    });

    window.onload = init;
