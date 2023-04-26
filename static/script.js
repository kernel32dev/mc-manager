
// GLOBALS //

let saves = {};
let selected = null;
let selected_elem = null;

// API FUNCTIONS //

function api(method, path, payload) {
    return new Promise((resolve, reject) => {
        let r = new XMLHttpRequest();
        r.open(method, path, true);
        r.onreadystatechange = function () {
            if (r.readyState !== 4) return;
            /** @type {any} */
            let response = r.responseText;
            if (response.length === 0) {
                response = {};
            } else {
                response = JSON.parse(response);
            }
            if (r.status === 200) {
                resolve(response);
            } else if (r.status === 400 || r.status === 500) {
                reject(response);
            } else {
                reject({
                    err: "BadStatus",
                    desc: "O Servidor retornou um status inesperado",
                    status: r.status,
                });
            }
        };
        if (typeof payload !== "string" && typeof payload !== undefined) {
            payload = JSON.stringify(payload);
        }
        r.send(payload);
    });
}

function api_list_versions() {
    return api("GET", "/api/versions", undefined);
}

function api_list_saves() {
    return api("GET", "/api/saves", undefined);
}

function api_create_save(name, version) {
    return api("POST", "/api/create_save", {name, version});
}

function api_modify_save(name, values) {
    return api("POST", "/api/create_save", {name, values});
}

function api_delete_save(name) {
    return api("POST", "/api/delete_save", {name});
}

// DOM FUNCTIONS //

function create_save_card(save) {
    let saves_container = document.getElementById("saves-container");
    let tr = document.createElement("tr");
    let td = document.createElement("td");
    let save_div = document.createElement("div");
    let img = document.createElement("img");
    let save_data = document.createElement("div");
    let save_line_1 = document.createElement("div");
    let save_line_2 = document.createElement("div");
    let save_line_3 = document.createElement("div");
    saves_container.append(tr);
    tr.append(td);
    td.append(save_div);
    save_div.append(img);
    save_div.append(save_data);
    save_data.append(save_line_1);
    save_data.append(save_line_2);
    save_data.append(save_line_3);
    img.setAttribute("src","/api/icons/" + save.name);
    save_div.dataset.name = save.name;
    save_div.classList.add("save");
    save_data.classList.add("save-data");
    save_line_1.classList.add("save-line", "save-line-1");
    save_line_2.classList.add("save-line", "save-line-2");
    save_line_3.classList.add("save-line", "save-line-3");
    save_line_1.innerText = save.name;
    save_line_2.innerText = "(" + save["mc-manager-create-time"] + ")";
    save_line_3.innerText = save["mc-manager-server-version"] + " - " + enum_gamemode(save["gamemode"]) + " Mode";
    save_div.addEventListener('click', function() {
        if (save_div.classList.contains("selected")) return;
        if (selected) {
            selected_elem.classList.remove("selected");
        } else {
            document.getElementById("button-play").classList.remove("disabled");
            document.getElementById("button-edit").classList.remove("disabled");
            document.getElementById("button-delete").classList.remove("disabled");
            document.getElementById("button-recreate").classList.remove("disabled");
            document.getElementById("button-restart").classList.remove("disabled");
        }
        save_div.classList.add("selected");
        selected = save.name;
        selected_elem = save_div;
    });
}

document.addEventListener("DOMContentLoaded", async function() {
    document.body.addEventListener("keydown", function(ev) {
        if (ev.key === "Escape" && selected) {
            selected_elem.classList.remove("selected");
            selected = null;
            selected_elem = null;
            document.getElementById("button-play").classList.add("disabled");
            document.getElementById("button-edit").classList.add("disabled");
            document.getElementById("button-delete").classList.add("disabled");
            document.getElementById("button-recreate").classList.add("disabled");
            document.getElementById("button-restart").classList.add("disabled");
        }
    })
    let response = await api_list_saves();
    console.log(response);
    let saves_container = document.getElementById("saves-container");
    saves_container.innerHTML = "";
    let saves_list = response.saves;
    for (let i = 0; i < saves_list.length; i++) {
        let save = saves_list[i];
        saves[save.name] = save;
        create_save_card(save);
    }
});

// HELPER FUNCTIONS //

function enum_gamemode(value) {
    if (value === 0) return "Survival";
    if (value === 1) return "Creative";
    if (value === 2) return "Adventure";
    if (value === 3) return "Spectator";
    return String(value);
}

function enum_difficulty(value) {
    if (value === 0) return "Peaceful";
    if (value === 1) return "Easy";
    if (value === 2) return "Medium";
    if (value === 3) return "Hard";
    return String(value);
}
