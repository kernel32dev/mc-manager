
document.addEventListener("DOMContentLoaded", main);

// GLOBALS //

let click_sound = null;

let schema = null;
let create_properties = null;

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
                try {
                    response = JSON.parse(response);
                } catch (e) {
                    console.log(response);
                    reject(e);
                    return;
                }
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

function api_fetch_versions() {
    return api("GET", "/api/versions", undefined);
}

function api_fetch_saves() {
    return api("GET", "/api/saves", undefined);
}

function api_fetch_schema() {
    return api("GET", "/api/schema", undefined);
}

function api_fetch_status() {
    return api("GET", "/api/status", undefined);
}

function api_create_save(name, version, values) {
    return api("POST", "/api/create_save", {name, version, values});
}

function api_modify_save(name, values) {
    return api("POST", "/api/modify_save", {name, values});
}

function api_delete_save(name) {
    return api("POST", "/api/delete_save", {name});
}

function api_start_save(name) {
    return api("POST", "/api/start_save", {name});
}

function api_stop_save(name) {
    return api("POST", "/api/stop_save", {name});
}

// DOM FUNCTIONS //

function create_saves(new_saves) {
    let old_selected = selected;
    unselect_save();
    saves = {};
    saves_elem = {};
    clear_elem(document.getElementById("saves-container"));
    foreach(new_saves, create_save);
    if (old_selected !== null && saves[old_selected] !== undefined) select_save(old_selected);
}

function create_save(save) {
    let name = save.name;
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
    img.setAttribute("src","/api/icons/" + name);
    save_div.dataset.name = name;
    save_div.classList.add("save", save.status);
    tr.classList.toggle("hide", !search_matches(name))
    save_data.classList.add("save-data");
    save_line_1.classList.add("save-line", "save-line-1");
    save_line_2.classList.add("save-line", "save-line-2");
    save_line_3.classList.add("save-line", "save-line-3");
    save_line_1.innerText = name;
    save_line_2.innerText = "(" + save["mc-manager-create-time"].substr(0, 16) + ")";
    save_line_3.innerText = save["mc-manager-server-version"] + " - " + ["Modo Sobrevivência", "Modo Criativo", "Modo Aventura", "Modo Spectador"][save["gamemode"]];
    save_div.addEventListener('click', function() {
        select_save(name);
    });
    save_div.addEventListener('dblclick', function() {
        if (save_div.classList.contains("offline")) {
            api_start_save(name).then(function(response) {
                save_div.classList.remove("offline");
                save_div.classList.add("loading");
                select_save(selected);
            }).catch(console.error);
        } else {
            api_stop_save(name).then(function(response) {
                save_div.classList.remove("loading", "online");
                save_div.classList.add("shutdown");
                select_save(selected);
            }).catch(console.error);
        }
    });
    saves[name] = save;
    saves_elem[name] = save_div;
}

function modify_save(name, values) {
    let save = saves[name];
    for (key in values) {
        save[key] = values[key];
    }
}

function delete_save(name) {
    if (selected === name) unselect_save();
    saves_elem[name].remove();
    delete saves[name];
    delete saves_elem[name];
}

function show_screen(screen) {
    const SCREENS = ["saves-screen", "create-screen", "modify-screen", "delete-screen"];
    if (selected === null && (screen === "modify-screen" || screen === "delete-screen")) {
        return;
    }
    for (let i = 0; i < SCREENS.length; i++) {
        let elements = document.getElementsByClassName(SCREENS[i]);
        let hide = SCREENS[i] !== screen;
        for (let j = 0; j < elements.length; j++) {
            elements[j].classList.toggle("hide", hide);
        }
    }
    if (screen === "create-screen") {
        unselect_save();
        document.getElementById("create-param-name").focus();
    } else if (screen === "modify-screen") {
        let save = saves[selected];
        foreach(document.getElementsByClassName("modify-param"), function(elem) {
            param_load(elem, save);
        });
    } else if (screen === "delete-screen") {
        let message = document.getElementById("delete-message-area");
        message.innerText = "Você tem certeza que quer apagar o mundo \"" + selected + "\"";
    }
}

function update_filter() {
    foreach(document.getElementsByClassName("save"), function(elem) {
        elem.parentElement.parentElement.classList.toggle("hide", !search_matches(elem.dataset.name));
    });
}

function update_status(status) {
    foreach(Object.values(saves), function(save) {
        if (save.status !== status[save.name]) {
            let elem = saves_elem[save.name];
            let new_status = status[save.name];
            if (new_status === undefined) {
                new_status = "offline";
            }
            save.status = new_status;
            elem.classList.remove("online", "offline", "loading", "shutdown");
            elem.classList.add(new_status);
            if (save.name === selected) {
                select_save(selected);
            }
        }
    });
}

function main() {

    // add sound to all buttons

    click_sound = document.createElement("audio");
    click_sound.src = "assets/click.mp3";
    click_sound.volume = 0.4;
    click_sound.load();
    foreach(document.getElementsByTagName("button"), function(button) {
        button.addEventListener("click", play_click_sound);
    });

    // initialize saves-screen

    document.body.addEventListener("keydown", function(ev) {
        if (ev.key === "Escape") unselect_save();
    });
    document.getElementById("saves-search").addEventListener("input", update_filter);
    document.getElementById("saves-button-play").addEventListener("click", function() {
        if (selected !== null) {
            let save_div = saves_elem[selected];
            if (save_div.classList.contains("offline")) {
                api_start_save(selected).then(function(response) {
                    save_div.classList.remove("offline");
                    save_div.classList.add("loading");
                    select_save(selected);
                }).catch(console.error);
            } else {
                api_stop_save(selected).then(function(response) {
                    save_div.classList.remove("loading", "online");
                    save_div.classList.add("shutdown");
                    select_save(selected);
                }).catch(console.error);
            }
        }
    });
    document.getElementById("saves-button-create").addEventListener("click", function() {
        show_screen("create-screen");
    });
    document.getElementById("saves-button-modify").addEventListener("click", function() {
        alert("TODO: ensure selected is offline");
        show_screen("modify-screen");
    });
    document.getElementById("saves-button-delete").addEventListener("click", function() {
        alert("TODO: ensure selected is offline");
        if (selected !== null) {
            show_screen("delete-screen");
        }
    });
    document.getElementById("saves-button-refresh").addEventListener("click", function() {
        api_fetch_saves().then(function(response) {
            create_saves(response.saves);
        }).catch(console.error);
    });
    document.getElementById("saves-button-log").addEventListener("click", function() {
        alert("TODO: create log")
    });
    api_fetch_saves().then(function(response) {
        create_saves(response.saves);
    }).catch(console.error);

    // initialize create-screen and modify-screen input fields

    api_fetch_schema().then(function(response) {
        schema = response.schema;
        create_properties = response.create_properties;
        let create_area = document.getElementById("create-input-area");
        foreach(create_properties, function(create_property) {
            let elem = param_setup(create_property);
            elem.classList.add("create-param", "wide");
            create_area.append(create_p(schema[create_property].label));
            create_area.append(elem);
        });
        create_area.append(create_p());
        let modify_area = document.getElementById("modify-input-area");
        let modify_properties = Object.keys(schema);
        modify_properties.sort();
        foreach(modify_properties, function(modify_property) {
            let elem = param_setup(modify_property);
            elem.classList.add("modify-param", "wide");
            modify_area.append(create_p(schema[modify_property].label));
            modify_area.append(elem);
        });
        modify_area.append(create_p());
    }).catch(console.error);

    // initialize create-screen buttons

    document.getElementById("create-input-search-version").addEventListener("click", function() {
        alert("TODO: add a version list");
    });
    document.getElementById("create-button-confirm").addEventListener("click", function() {
        let name_elem = document.getElementById("create-param-name");
        let version_elem = document.getElementById("create-param-version");
        if (!validate_name(name_elem)) return;
        if (!validate_version(version_elem)) return;
        let name = name_elem.value.trim();
        let version = version_elem.value.trim();
        let values = param_values("create-param");
        if (values === null) return;
        disable("create-button-confirm", "create-button-cancel");
        api_create_save(name, version, values).then(function(response) {
            enable("create-button-confirm", "create-button-cancel");
            create_save(response);
            select_save(response.name);
            show_screen("saves-screen");
        }).catch(function(response) {
            enable("create-button-confirm", "create-button-cancel");
            console.error(response);
        });
    });

    // initialize modify-screen buttons

    document.getElementById("modify-button-confirm").addEventListener("click", function() {
        let values = param_values("modify-param");
        if (values === null) return;
        disable("modify-button-confirm", "modify-button-cancel");
        let name = selected;
        api_modify_save(name, values).then(function(response) {
            enable("modify-button-confirm", "modify-button-cancel");
            modify_save(name, values);
            show_screen("saves-screen");
        }).catch(function(response) {
            enable("modify-button-confirm", "modify-button-cancel");
            console.error(response);
        });
    });

    // initialize delete-screen buttons

    document.getElementById("delete-button-confirm").addEventListener("click", function() {
        if (selected !== null) {
            disable("delete-button-confirm", "delete-button-cancel");
            let name = selected;
            api_delete_save(name).then(function() {
                enable("delete-button-confirm", "delete-button-cancel");
                delete_save(name);
                show_screen("saves-screen");
            }).catch(function(response) {
                enable("delete-button-confirm", "delete-button-cancel");
                console.error(response);
            });
        } else {
            show_screen("saves-screen");
        }
    });

    // setup status update

    setInterval(function() {
        api_fetch_status().then(update_status).catch(console.error);
    }, 1000);
}

// HELPER FUNCTIONS //

function select_save(name) {
    if (selected === name) {
        if (selected !== null) {
            document.getElementById("saves-button-play").firstElementChild.innerText = saves_elem[selected].classList.contains("offline") ? "Iniciar mundo" : "Parar mundo";
        }
        return;
    }
    if (!search_matches(name)) {
        unselect_save();
        return;
    }
    if (selected === null) {
        enable("saves-button-play", "saves-button-modify", "saves-button-delete", "saves-button-log");
    } else {
        saves_elem[selected].classList.remove("selected");
    }
    let elem = saves_elem[name];
    elem.classList.add("selected");
    document.getElementById("saves-button-play").firstElementChild.innerText = elem.classList.contains("offline") ? "Iniciar mundo" : "Parar mundo";
    selected = name;
}

function unselect_save() {
    if (selected === null) return;
    saves_elem[selected].classList.remove("selected");
    selected = null;
    document.getElementById("saves-button-play").firstElementChild.innerText = "Iniciar mundo";
    disable("saves-button-play", "saves-button-modify", "saves-button-delete", "saves-button-log");
}

// creates an element that accepts typed input from the user
function param_setup(prop_name) {
    let prop = schema[prop_name];
    let elem;
    if (prop.type.name === "boolean") {
        elem = document.createElement("button");
        let span = document.createElement("span");
        elem.dataset.value = prop.type["default"];
        span.innerText = prop.type["default"] ? "Sim" : "Não";
        elem.addEventListener("click", function() {
            play_click_sound();
            if (elem.dataset.value === "0") {
                elem.dataset.value = "1";
                elem.firstElementChild.innerText = "Sim";
            } else {
                elem.dataset.value = "0";
                elem.firstElementChild.innerText = "Não";
            }
        });
        elem.append(span);
    } else if (prop.type.name === "integer-enum") {
        elem = document.createElement("button");
        let span = document.createElement("span");
        elem.dataset.value = prop.type["default"];
        span.innerText = prop.type.members[prop.type["default"]];
        elem.addEventListener("click", function() {
            play_click_sound();
            let value = Number(elem.dataset.value) + 1;
            if (value >= prop.type.members.length) value = 0;
            elem.firstElementChild.innerText = prop.type.members[value];
            elem.dataset.value = value;
        });
        elem.append(span);
    } else if (prop.type.name === "string-enum") {
        elem = document.createElement("button");
        let span = document.createElement("span");
        elem.dataset.value = prop.type["default"];
        span.innerText = prop.type.members[prop.type["default"]];
        elem.addEventListener("click", function() {
            play_click_sound();
            let value = Number(elem.dataset.value) + 1;
            if (value >= prop.type.members.length) value = 0;
            elem.firstElementChild.innerText = prop.type.members[value][0];
            elem.dataset.value = value;
        });
        elem.append(span);
    } else {
        elem = document.createElement("input");
        elem.value = prop.type["default"];
    }
    elem.classList.toggle("disabled", prop.access !== "write");
    elem.dataset.prop = prop_name;
    return elem;
}

// calls param_value for all elems, and returns null if one fails
// if elems is a string, all elems of that class name are used instead
function param_values(elems) {
    if (typeof elems === "string") {
        elems = document.getElementsByClassName(elems);
    }
    let values = {};
    for (let i = 0; i < elems.length; i++) {
        if (!param_value(elems[i], values)) {
            return null;
        }
    }
    return values;
}

// takes a param elem, validates its value, and adds its value to the values object
// returns false if validation fails
function param_value(elem, values) {
    if (elem.classList.contains("disabled")) {
        return true;
    }
    let prop = schema[elem.dataset.prop];
    let value;
    if (elem.tagName === "BUTTON") {
        if (prop.type.name === "boolean") {
            value = elem.dataset.value !== "0";
        } else if (prop.type.name === "integer-enum") {
            value = Number(elem.dataset.value);
        } else if (prop.type.name === "string-enum") {
            value = prop.type.members[Number(elem.dataset.value)][1];
        } else {
            throw new Error("unknown property type for button " + prop.type.name);
        }
    } else if (elem.tagName === "INPUT") {
        if (prop.type.name === "integer") {
            if (!validate_integer(elem, prop.type.min, prop.type.max)) {
                elem.focus();
                return false;
            }
            value = Number(elem.value.trim());
        } else {
            value = elem.value.trim();
        }
    }
    values[elem.dataset.prop] = value;
    return true;
}

// takes the value from the save into the element
function param_load(elem, save) {
    let prop = schema[elem.dataset.prop];
    let value = save[elem.dataset.prop];
    if (elem.tagName === "BUTTON") {
        if (prop.type.name === "boolean") {
            if (value) {
                elem.dataset.value = "1";
                elem.firstElementChild.innerText = "Sim";
            } else {
                elem.dataset.value = "0";
                elem.firstElementChild.innerText = "Não";
            }
        } else if (prop.type.name === "integer-enum") {
            elem.dataset.value = value;
            elem.firstElementChild.innerText = prop.type.members[value];
        } else if (prop.type.name === "string-enum") {
            let index = 0;
            for (; index < prop.type.members.length; index++) {
                if (prop.type.members[index][1] === value) {
                    break;
                }
            }
            if (index >= prop.type.members.length) {
                console.warn(`member ${value} of enum for ${elem.dataset.prop} not found, using default (${prop.type.members[prop.type["default"]][1]}) instead`);
                index = prop.type["default"];
            }
            elem.dataset.value = index;
            elem.firstElementChild.innerText = prop.type.members[index][0];
        } else {
            throw new Error("unknown property type for button " + prop.type.name);
        }
    } else if (elem.tagName === "INPUT") {
        elem.value = value;
    }
}

const FOR_EACH_BREAK = {};

// class callback for each item in array, if the callback returns FOR_EACH_BREAK, the for quits early
function foreach(array, callback) {
    for (let i = 0; i < array.length; i++) {
        if (callback(array[i], i, array) === FOR_EACH_BREAK) break;
    }
}

// removes the disabled class to all elements/ids passed as arguments
function enable() {
    for (let i = 0; i < arguments.length; i++) {
        let arg = arguments[i];
        if (typeof arg === "string") {
            document.getElementById(arg).classList.remove("disabled");
        } else if (typeof arg === "object") {
            arg.classList.remove("disabled");
        }
    }
}

// adds the disabled class to all elements/ids passed as arguments
function disable() {
    for (let i = 0; i < arguments.length; i++) {
        let arg = arguments[i];
        if (typeof arg === "string") {
            document.getElementById(arg).classList.add("disabled");
        } else if (typeof arg === "object") {
            arg.classList.add("disabled");
        }
    }
}

// removes all children
function clear_elem(elem) {
    elem.innerHTML = "";
    if (elem.firstChild) {
        elem.firstChild.remove();
    }
}

function play_click_sound() {
    try {
        click_sound.pause();
        click_sound.currentTime = 0;
        click_sound.play().catch(console.error);
    } catch (e) {
        console.error(e);
    }
}

function create_p(text) {
    let p = document.createElement("p");
    if (typeof text === "string") {
        p.innerText = text;
    } else {
        p.innerHTML = "&nbsp;";
    }
    return p;
}

// returns true if with the current search this item would be found
function search_matches(item) {
    let search = document.getElementById("saves-search").value.trim().toLocaleLowerCase();
    let words = search.split(/ +/g);
    item = item.toLocaleLowerCase();
    return words.some(function(word) { return item.indexOf(word) !== -1; });
}

function validate_name(elem) {
    return true;
}

function validate_version(elem) {
    return true;
}

function validate_integer(elem, min, max) {
    if (!/^-?\d+$/.test(elem.value.trim())) {
        return false;
    }
    let number = Number(elem.value);
    if (typeof min === "number" && number < min) {
        return false;
    }
    if (typeof max === "number" && number > max) {
        return false;
    }
    return true;
}
