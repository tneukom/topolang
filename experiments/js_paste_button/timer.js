window.onload = function() {
    setTimeout(function() {
        navigator.clipboard.readText()
            .then(text => {
                document.getElementById('myTextField').value = text;
            })
            .catch(err => {
                console.error('Failed to read clipboard contents: ', err);
            });
    }, 1000);
};
