// Shared fetch helper — redirects to /login on 401
function apiFetch(url, options) {
    return fetch(url, options).then(r => {
        if (r.status === 401) {
            window.location.href = '/login';
            return new Promise(() => {}); // stop the chain silently
        }
        if (!r.ok) return r.json().then(d => Promise.reject(d.error || 'HTTP ' + r.status));
        return r.json();
    });
}
