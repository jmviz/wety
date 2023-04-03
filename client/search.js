const searchInput = document.getElementById('search-input');
const suggestionsList = document.getElementById('suggestions-list');
let suggestions = [];
let selectedSuggestionIndex = -1;

let timeoutId;

const api = window.location.hostname === 'localhost' ? 'http://localhost:3000/' : 'https://api.wety.org/';

function displayMatches(matches) {
    suggestionsList.innerHTML = '';
    suggestions = matches;
    selectedSuggestionIndex = -1;
    if (suggestions.length === 0) {
        suggestionsList.style.display = 'none';
        return;
    }
    suggestionsList.style.display = 'block';
    suggestions.forEach((suggestion, index) => {
        const li = document.createElement('li');
        li.classList.add('suggestion-item');
        li.textContent = suggestion.lang;
        li.addEventListener('click', () => {
            searchInput.value = suggestion.lang;
            suggestionsList.style.display = 'none';
        });
        suggestionsList.appendChild(li);
        if (index === selectedSuggestionIndex) {
            li.classList.add('selected');
        }
    });
}

async function fetchSuggestions() {
    const input = encodeURIComponent(searchInput.value.toLowerCase());
    if (!input) {
        suggestionsList.innerHTML = '';
        return;
    }
    try {
        const response = await fetch(`${api}langs/${input}`);
        const data = await response.json();
        console.log(data);
        const matches = data.matches;
        displayMatches(matches);
    } catch (error) {
        console.error(error);
    }
}

searchInput.addEventListener('input', () => {
    clearTimeout(timeoutId);
    timeoutId = setTimeout(fetchSuggestions, 500);
});

searchInput.addEventListener('keydown', event => {
    if (event.key === 'ArrowDown') {
        event.preventDefault();
        if (selectedSuggestionIndex < suggestions.length - 1) {
            selectedSuggestionIndex++;
            updateSelectedSuggestion();
        }
    } else if (event.key === 'ArrowUp') {
        event.preventDefault();
        if (selectedSuggestionIndex > -1) {
            selectedSuggestionIndex--;
            updateSelectedSuggestion();
        }
    } else if (event.key === 'Tab' || event.key === 'Enter') {
        if (selectedSuggestionIndex > -1) {
            event.preventDefault();
            searchInput.value = suggestions[selectedSuggestionIndex].lang;
            suggestionsList.style.display = 'none';
            selectedSuggestionIndex = -1;
        }
    }
});

function updateSelectedSuggestion() {
    const suggestionElements = suggestionsList.getElementsByTagName('li');
    for (let i = 0; i < suggestionElements.length; i++) {
        const suggestionElement = suggestionElements[i];
        if (i === selectedSuggestionIndex) {
            suggestionElement.classList.add('selected');
            const elementRect = suggestionElement.getBoundingClientRect();
            const containerRect = suggestionsList.getBoundingClientRect();
            if (elementRect.bottom > containerRect.bottom) {
                suggestionsList.scrollTop += elementRect.bottom - containerRect.bottom;
            } else if (elementRect.top < containerRect.top) {
                suggestionsList.scrollTop -= containerRect.top - elementRect.top;
            }
        } else {
            suggestionElement.classList.remove('selected');
        }
    }
}