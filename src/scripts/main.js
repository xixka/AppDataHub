/**
 * AppDataHub — 前端逻辑
 */

const { invoke } = window.__TAURI__;

const $ = (id) => document.getElementById(id);

const els = {
    accountList: $('account-list'),
    emptyState: $('empty-state'),
    btnAdd: $('btn-add-account'),
    btnSettings: $('btn-settings'),
    btnRefreshStatus: $('btn-refresh-status'),
    profileConfigPath: $('profile-config-path'),
    appRunningStatus: $('app-running-status'),
    modalAccount: $('modal-account'),
    modalTitle: $('modal-title'),
    inputName: $('input-name'),
    inputEmail: $('input-email'),
    inputNote: $('input-note'),
    modalClose: $('modal-close'),
    modalCancel: $('modal-cancel'),
    modalSave: $('modal-save'),
    modalSettings: $('modal-settings'),
    settingsClose: $('settings-close'),
    settingsCancel: $('settings-cancel'),
    inputConfigDir: $('input-config-dir'),
    inputDataDir: $('input-data-dir'),
    btnDetectPaths: $('btn-detect-paths'),
    toastContainer: $('toast-container'),
};

let editingAccountId = null;

// ===== Toast =====
function toast(message, type = 'info') {
    const el = document.createElement('div');
    el.className = `toast ${type}`;
    el.textContent = message;
    els.toastContainer.appendChild(el);
    setTimeout(() => {
        el.style.opacity = '0';
        setTimeout(() => el.remove(), 300);
    }, 3000);
}

// ===== 账号列表 =====
async function loadAccounts() {
    try {
        const accounts = await invoke('list_accounts');
        renderAccounts(accounts);
    } catch (e) {
        toast('加载账号失败: ' + e, 'error');
    }
}

function renderAccounts(accounts) {
    if (!accounts || accounts.length === 0) {
        els.accountList.innerHTML = '';
        els.emptyState.style.display = 'flex';
        return;
    }
    els.emptyState.style.display = 'none';
    els.accountList.innerHTML = accounts.map(acc => `
        <div class="account-card ${acc.is_active ? 'active' : ''}" data-id="${acc.id}">
            <div class="account-info">
                <div class="account-name">
                    ${acc.is_active ? '<span class="badge-active">当前</span>' : ''}
                    ${escapeHtml(acc.name)}
                </div>
                <div class="account-meta">
                    ${acc.email ? escapeHtml(acc.email) : ''}
                    ${acc.email && acc.last_used ? ' · ' : ''}
                    ${acc.last_used ? formatDate(acc.last_used) : ''}
                    ${!acc.has_snapshot ? ' · <span style="color:#d97706">无快照</span>' : ''}
                </div>
                ${acc.note ? `<div class="account-note">${escapeHtml(acc.note)}</div>` : ''}
            </div>
            <div class="account-actions">
                ${!acc.is_active ? `<button class="btn btn-primary btn-sm btn-switch" data-id="${acc.id}">切换</button>` : ''}
                <button class="btn btn-ghost btn-sm btn-save" data-id="${acc.id}" title="保存当前配置到此账号">
                    ${acc.has_snapshot ? '更新' : '保存'}
                </button>
                <button class="btn btn-ghost btn-sm btn-edit" data-id="${acc.id}">编辑</button>
                <button class="btn btn-ghost btn-sm btn-delete" data-id="${acc.id}" style="color:#dc2626">删除</button>
            </div>
        </div>
    `).join('');

    els.accountList.querySelectorAll('.btn-switch').forEach(btn => {
        btn.onclick = () => switchAccount(btn.dataset.id);
    });
    els.accountList.querySelectorAll('.btn-save').forEach(btn => {
        btn.onclick = () => saveSnapshot(btn.dataset.id);
    });
    els.accountList.querySelectorAll('.btn-edit').forEach(btn => {
        btn.onclick = () => openEditModal(btn.dataset.id);
    });
    els.accountList.querySelectorAll('.btn-delete').forEach(btn => {
        btn.onclick = () => deleteAccount(btn.dataset.id);
    });
}

function escapeHtml(str) {
    const div = document.createElement('div');
    div.textContent = str;
    return div.innerHTML;
}

function formatDate(dateStr) {
    const d = new Date(dateStr);
    return `${d.getMonth()+1}月${d.getDate()}日 ${String(d.getHours()).padStart(2,'0')}:${String(d.getMinutes()).padStart(2,'0')}`;
}

// ===== 账号操作 =====
async function switchAccount(id) {
    const running = await invoke('check_app_running');
    if (running) {
        toast('目标应用正在运行，请先关闭再切换', 'error');
        return;
    }
    if (!confirm('确定切换到此账号？这将覆盖当前配置。')) return;
    try {
        await invoke('switch_account', { id });
        toast('切换成功', 'success');
        await loadAccounts();
    } catch (e) {
        toast('切换失败: ' + e, 'error');
    }
}

async function saveSnapshot(id) {
    try {
        await invoke('save_current_snapshot', { id });
        toast('配置已保存', 'success');
        await loadAccounts();
    } catch (e) {
        toast('保存失败: ' + e, 'error');
    }
}

async function deleteAccount(id) {
    if (!confirm('确定删除此账号？此操作不可撤销。')) return;
    try {
        await invoke('delete_account', { id });
        toast('已删除', 'success');
        await loadAccounts();
    } catch (e) {
        toast('删除失败: ' + e, 'error');
    }
}

// ===== 弹窗 =====
function openAddModal() {
    editingAccountId = null;
    els.modalTitle.textContent = '添加账号';
    els.inputName.value = '';
    els.inputEmail.value = '';
    els.inputNote.value = '';
    els.modalAccount.style.display = 'flex';
    els.inputName.focus();
}

async function openEditModal(id) {
    try {
        const accounts = await invoke('list_accounts');
        const acc = accounts.find(a => a.id === id);
        if (!acc) return;
        editingAccountId = id;
        els.modalTitle.textContent = '编辑账号';
        els.inputName.value = acc.name;
        els.inputEmail.value = acc.email || '';
        els.inputNote.value = acc.note || '';
        els.modalAccount.style.display = 'flex';
        els.inputName.focus();
    } catch (e) {
        toast('加载失败: ' + e, 'error');
    }
}

function closeAccountModal() {
    els.modalAccount.style.display = 'none';
    editingAccountId = null;
}

async function saveAccount() {
    const name = els.inputName.value.trim();
    if (!name) { toast('请输入账号名称', 'error'); return; }
    const email = els.inputEmail.value.trim() || null;
    const note = els.inputNote.value.trim() || null;
    try {
        if (editingAccountId) {
            await invoke('update_account', {
                id: editingAccountId,
                name,
                email,
                note,
            });
            toast('已更新', 'success');
        } else {
            await invoke('add_account', { name, email, note });
            toast('已添加', 'success');
        }
        closeAccountModal();
        await loadAccounts();
    } catch (e) {
        toast('保存失败: ' + e, 'error');
    }
}

// ===== 设置 =====
async function loadProfileInfo() {
    try {
        const info = await invoke('get_profile_info');
        els.profileConfigPath.textContent = info.config_dir;
        els.inputConfigDir.value = info.config_dir;
        els.inputDataDir.value = info.user_dir || '(无)';
    } catch (e) {
        els.profileConfigPath.textContent = '未检测到';
    }
}

async function checkAppRunning() {
    try {
        const running = await invoke('check_app_running');
        if (running) {
            els.appRunningStatus.innerHTML = '<span style="color:#dc2626">● 运行中</span>';
        } else {
            els.appRunningStatus.innerHTML = '<span style="color:#16a34a">● 未运行</span>';
        }
    } catch (e) {
        els.appRunningStatus.textContent = '未知';
    }
}

function openSettings() {
    els.modalSettings.style.display = 'flex';
}

function closeSettings() {
    els.modalSettings.style.display = 'none';
}

async function detectPaths() {
    try {
        const info = await invoke('detect_profile');
        els.inputConfigDir.value = info.config_dir;
        els.inputDataDir.value = info.user_dir || '(无)';
        toast('检测完成', 'success');
    } catch (e) {
        toast('检测失败: ' + e, 'error');
    }
}

// ===== 事件绑定 =====
els.btnAdd.onclick = openAddModal;
els.btnSettings.onclick = openSettings;
els.btnRefreshStatus.onclick = () => { checkAppRunning(); loadProfileInfo(); };
els.modalClose.onclick = closeAccountModal;
els.modalCancel.onclick = closeAccountModal;
els.modalSave.onclick = saveAccount;
els.modalAccount.onclick = (e) => { if (e.target === els.modalAccount) closeAccountModal(); };
els.settingsClose.onclick = closeSettings;
els.settingsCancel.onclick = closeSettings;
els.btnDetectPaths.onclick = detectPaths;
els.modalSettings.onclick = (e) => { if (e.target === els.modalSettings) closeSettings(); };
els.inputName.onkeydown = (e) => { if (e.key === 'Enter') saveAccount(); };

// ===== 初始化 =====
async function init() {
    await loadProfileInfo();
    await checkAppRunning();
    await loadAccounts();
    setInterval(checkAppRunning, 30000);
}

init();
