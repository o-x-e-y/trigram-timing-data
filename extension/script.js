let max_trigram_delay = 750;
let save_cooldown = 5000;

let trigram_data = new Map();
let last_keypresses = [];
let last_timestamps = [];
let last_save = 0;

if (localStorage.getItem("trigram_data") === null) {
    localStorage.setItem("trigram_data", "{}");
}

document.addEventListener("keydown", event => {
    const current_time = performance.now();
    last_keypresses.push(event.code);
    last_timestamps.push(current_time);
    
    if (last_keypresses.length > 3) {
        last_keypresses.shift();
        last_timestamps.shift();
    }

    if (last_keypresses.length == 3) {
        const trigram_time = last_timestamps[2] - last_timestamps[0];
        const trigram = `${last_keypresses[0]},${last_keypresses[1]},${last_keypresses[2]}`;

        if (trigram_time < max_trigram_delay) {
            if (!trigram_data.has(trigram)) {
                trigram_data.set(trigram, []);
            }
    
            trigram_data.get(trigram).push(trigram_time);
        } else {
            console.log(`skipped [${trigram}] for time ${trigram_time}`);
        }
    }

    if (current_time - last_save > save_cooldown && event.code == "Tab") {
        last_save = current_time;

        combine_data();

        console.log("saved data in localStorage under key \"trigram_data\"");
    } else if (event.code === "Tab") {
        console.log(`didn't save because it's only been ${current_time - last_save} ms since last save.`);
    }
});

function combine_data() {
    let last_data = JSON.parse(localStorage.getItem("trigram_data"));
    let current_data = convert_data();

    for(let t of current_data.keys()) {
        last_data[t] = add_datapoint(current_data.get(t), last_data[t]);
    }

    let s = JSON.stringify(last_data);
    localStorage.setItem("trigram_data", s);

    trigram_data.clear()

    return last_data
}

function add_datapoint(current, last) {
    if (current === undefined || current === null) {
        console.log(`returned last because current is ${current}`);
        return [...last];
    
    } else if (last === undefined || current === null) {
        console.log(`returned current because last is ${last}`);
        return [...current];
    
    } else {
        return current.concat(last)
    }
}

function convert_data() {
    return structuredClone(trigram_data);
}

function get_trigram_data() {
    let obj = combine_data();
    // let obj = Object.fromEntries(trigrams_to_save);
    let s = JSON.stringify(obj, null, '\t');

    let re1 = new RegExp("[\\n\\s]+([\\d\\.]+,?)(\\n\\t)?", "g");
    let re2 = new RegExp(" \\]", "g");

    s = s.replace(re1, "$1 ");
    s = s.replace(re2, "]");

    console.log(s);
}

function reset_trigram_data() {
    trigram_data.clear();
    localStorage.setItem("trigram_data", "{}");
}