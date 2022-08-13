const domContainer = document.querySelector('#Main');

fetch('/api/categories').then(res => res.json()).then(data =>
    ReactDOM.render(e(MainView, {"categories": data, "entries": []}), domContainer));
