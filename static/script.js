
// GLOBALS //

let saves = {};
let saves_elem = {};
let selected = null;

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
        if (payload === null || payload === undefined) {
            r.send();
        } else if (typeof payload === "string") {
            r.send(payload);
        } else if (typeof payload == "object" && payload !== null) {
            r.setRequestHeader("Content-Type", "application/json");
            r.send(JSON.stringify(payload));
        } else {
            console.error("invalid payload type " + typeof payload);
        }
    });
}

function api_list_versions() {
    return api("GET", "/api/versions", undefined);
}

function api_list_saves() {
    return api("GET", "/api/saves", undefined);
}

function api_create_save(name, version, values) {
    return api("POST", "/api/create_save", {name, version, values});
}

function api_modify_save(name, values) {
    return api("POST", "/api/create_save", {name, values});
}

function api_delete_save(name) {
    return api("POST", "/api/delete_save", {name});
}

// DOM FUNCTIONS //

function create_save(save) {
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
    save_line_2.innerText = "(" + save["mc-manager-create-time"].substr(0, 16) + ")";
    save_line_3.innerText = save["mc-manager-server-version"] + " - " + ["Modo Sobrevivência", "Modo Criativo", "Modo Aventura", "Modo Spectador"][save["gamemode"]];
    save_div.addEventListener('click', function() {
        select_save(save.name);
    });
    saves[save.name] = save;
    saves_elem[save.name] = save_div;
    return save_div;
}

document.addEventListener("DOMContentLoaded", function() {
    // add sound to all buttons
    let click_sound = createAudio("assets/click.mp3", 0.4);
    forEach(document.getElementsByTagName("button"), function(button) {
        button.addEventListener("click", click_sound);
    });
    // initialize saves-screen
    document.body.addEventListener("keydown", function(ev) {
        if (ev.key === "Escape") unselect_save();
    });
    document.getElementById("saves-button-delete").addEventListener("click", function() {
        if (selected === null) return;
        let name = selected;
        api_delete_save(name).then(function() {
            if (selected === name) unselect_save();
            saves_elem[name].remove();
            delete saves[name];
            delete saves_elem[name];
        }).catch(console.error);
    });
    api_list_saves().then(function(response) {
        document.getElementById("saves-container").innerHTML = "";
        forEach(response.saves, create_save);
    });

    // initialize create-screen

    // enums for button properties
    const ENUMS = {
        "gamemode": ["Modo Sobrevivência", "Modo Criativo", "Modo Aventura", "Modo Spectador"],
        "difficulty": ["Pacífico", "Fácil", "Médio", "Difícil"],
        "boolean": ["Não", "Sim"],
        "level-type": ["Normal", "Plano", "Grandes Biomas", "Aplificado"]
    };

    // validators for input properties
    const VALIDATORS = {
        text(elem) {
            return true;
        },
        int(elem) {
            if (!/^\d+$/.test(elem.value.trim())) {
                return false;
            }
            let number = Number(elem.value);
            if (typeof elem.dataset.min === "string" && number < Number(elem.dataset.min)) {
                return false;
            }
            if (typeof elem.dataset.max === "string" && number > Number(elem.dataset.max)) {
                return false;
            }
            return true;
        },
    };

    forEach(document.getElementsByClassName("create-param"), function(elem) {
        if (elem.tagName === "BUTTON") {
            elem.addEventListener("click", function() {
                let value = Number(elem.dataset.value) + 1;
                let enum_list = ENUMS[elem.dataset.type];
                if (value >= enum_list.length) value = 0;
                elem.firstElementChild.innerText = enum_list[value];
                elem.dataset.value = value;
            });
        }
    });
    document.getElementById("create-input-search-version").addEventListener("click", function() {
        alert("TODO: add a version list");
    });
    document.getElementById("create-button-confirm").addEventListener("click", function() {
        let name_elem = document.getElementById("create-param-name");
        let name_version = document.getElementById("create-param-version");
        let name = name_elem.value.trim();
        // TODO: validate name
        let version = name_version.value.trim();
        // TODO: validate version
        let values = {};
        forEach(document.getElementsByClassName("create-param"), function(param_elem) {
            /** @type {HTMLElement} */ 
            let elem = param_elem;
            if (elem.tagName === "BUTTON") {
                if (elem.dataset.type === "boolean") {
                    values[elem.dataset.prop] = elem.dataset.value !== "0";
                } else {
                    values[elem.dataset.prop] = Number(elem.dataset.value);
                }
            } else if (elem.tagName === "INPUT") {
                if (!VALIDATORS[elem.dataset.type](elem)) {
                    elem.focus();
                    values = null;
                    return FOR_EACH_BREAK;
                }
                if (elem.dataset.type === "int") {
                    values[elem.dataset.prop] = Number(elem.value);
                } else {
                    values[elem.dataset.prop] = elem.value.trim();
                }
            }
        });
        if (values === null) return;
        if (values["level-type"] !== undefined) {
            values["level-type"] = ["normal", "flat", "large_biomes", "amplified"][values["level-type"]];
        }
        let confirm_button = document.getElementById("create-button-confirm");
        let cancel_button = document.getElementById("create-button-cancel");
        confirm_button.classList.add("disabled");
        cancel_button.classList.add("disabled");
        api_create_save(name, version, values).then(function(response) {
            console.log("success", response);
            confirm_button.classList.remove("disabled");
            cancel_button.classList.remove("disabled");
            create_save(response);
            select_save(response.name);
            show_screen("saves-screen");
        }).catch(function(response) {
            console.log("error", response);
            confirm_button.classList.remove("disabled");
            cancel_button.classList.remove("disabled");

        });
    });
});

// HELPER FUNCTIONS //

function show_screen(screen) {
    const SCREENS = ["saves-screen", "create-screen"];
    for (let i = 0; i < SCREENS.length; i++) {
        let elements = document.getElementsByClassName(SCREENS[i]);
        let hide = SCREENS[i] !== screen;
        for (let j = 0; j < elements.length; j++) {
            elements[j].classList.toggle("hide", hide);
        }
    }
    if (screen === "create-screen") {
        document.getElementById("create-param-name").focus();
    }
}

function select_save(name) {
    if (selected === name) return;
    if (selected === null) {
        document.getElementById("saves-button-play").classList.remove("disabled");
        document.getElementById("saves-button-edit").classList.remove("disabled");
        document.getElementById("saves-button-delete").classList.remove("disabled");
        document.getElementById("saves-button-recreate").classList.remove("disabled");
        document.getElementById("saves-button-restart").classList.remove("disabled");
    } else {
        saves_elem[selected].classList.remove("selected");
    }
    saves_elem[name].classList.add("selected");
    selected = name;
}

function unselect_save() {
    if (selected === null) return;
    saves_elem[selected].classList.remove("selected");
    selected = null;
    document.getElementById("saves-button-play").classList.add("disabled");
    document.getElementById("saves-button-edit").classList.add("disabled");
    document.getElementById("saves-button-delete").classList.add("disabled");
    document.getElementById("saves-button-recreate").classList.add("disabled");
    document.getElementById("saves-button-restart").classList.add("disabled");
}

const FOR_EACH_BREAK = false;

function forEach(array, callback) {
    for (let i = 0; i < array.length; i++) {
        if (callback(array[i], i, array) === false) break;
    }
}

function createAudio(src, volume) {
    let audio = document.createElement("audio");
    audio.src = src;
    audio.volume = volume;
    audio.load();
    return function() {
        try {
            audio.pause();
            audio.currentTime = 0;
            audio.play().catch(console.error);
        } catch (e) {
            console.error(e);
        }
    }
}
