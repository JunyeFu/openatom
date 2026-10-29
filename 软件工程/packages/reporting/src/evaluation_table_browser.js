const tableStates = new Map();
const filter = document.getElementById('filter');
function renderTable(details) {
  let state = tableStates.get(details);
  if (!state) {
    state = { rows: JSON.parse(details.querySelector('script').textContent), page: 0 };
    tableStates.set(details, state);
  }
  const query = filter.value.toLocaleLowerCase();
  const rows = query ? state.rows.filter(row => row.some(value => (value ?? '未知/不适用').toLocaleLowerCase().includes(query))) : state.rows;
  const pages = Math.max(1, Math.ceil(rows.length / 100));
  state.page = Math.min(state.page, pages - 1);
  const body = details.querySelector('tbody');
  body.replaceChildren();
  for (const row of rows.slice(state.page * 100, state.page * 100 + 100)) {
    const tr = document.createElement('tr');
    tr.className = 'data-row';
    for (const value of row) {
      const td = document.createElement('td');
      td.textContent = value ?? '未知/不适用';
      tr.append(td);
    }
    body.append(tr);
  }
  details.querySelector('.table-page').textContent = `第 ${state.page + 1} / ${pages} 页 · 筛选后 ${rows.length} 条，每页100条`;
  details.querySelector('.table-previous').disabled = state.page === 0;
  details.querySelector('.table-next').disabled = state.page + 1 >= pages;
}
for (const details of document.querySelectorAll('.data-table')) {
  details.addEventListener('toggle', () => { if (details.open) renderTable(details); });
  details.querySelector('.table-previous').addEventListener('click', () => { tableStates.get(details).page--; renderTable(details); });
  details.querySelector('.table-next').addEventListener('click', () => { tableStates.get(details).page++; renderTable(details); });
}
filter.addEventListener('input', () => {
  for (const state of tableStates.values()) state.page = 0;
  for (const details of document.querySelectorAll('.data-table[open]')) renderTable(details);
});
document.getElementById('raw-data').addEventListener('toggle', event => {
  const raw = document.getElementById('raw');
  if (event.currentTarget.open && !raw.textContent) raw.textContent = JSON.stringify(JSON.parse(document.getElementById('report-data').textContent), null, 2);
});
