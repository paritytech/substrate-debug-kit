var blessed = require('blessed'), fs = require('fs'), child_process = require('child_process');

const TITLE_PROG="program:";

fs.readFile(process.argv[2], function(err,data){
    var js = JSON.parse(data)
    var screen = blessed.screen({
        autoPadding: true,
        smartCSR: true
    });
    var form = blessed.form({
        parent: screen,
        width: 60,
        height: 4,
        keys: true
    });
    blessed.text({
        parent: form,
        fg: 'cyan',
        content: TITLE_PROG
    })
    var prog = blessed.textbox({
        parent: form,
        name:'program',
        inputOnFocus: true,
        value: js['program'],
        left: TITLE_PROG.length + 1
    })
    var texts = Object.keys(js['args']).forEach(function(key,index){
        blessed.text({
            parent: form,
            top: index + 1,
            content: key + ':',
            fg:'green'
        })
        blessed.textbox({
            parent: form,
            inputOnFocus: true,
            name: key,
            value: js['args'][key],
            top: index + 1,
            left: key.length + 2
        })
    })

    form.on('submit', function(data){
        screen.leave();
        var prog = data['program'] 
        delete data['program']
        var cmd = prog + ' ' + Object.keys(data).map(function(key){return '-' + key + ' "' + data[key] + '"'}).join(' ')
        child_process.exec(cmd,function(error,stdout,stderr){
            screen.leave();
            console.log('stdout: ' + stdout)
            console.log('stderr: ' + stderr)
            if(error !== null){
                console.log('error: ' + error)
                process.exit(error.code);
            }
            process.exit(0);
        })
    })
    screen.key(['enter'], function(){
        form.submit();
    });

    screen.key(['escape','C-c'], function(){
        screen.leave();
        process.exit(0);
    });

    prog.focus()

    screen.render();

})
