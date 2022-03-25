const e = React.createElement;

const data = {"name": "root",
              "key": 0,
              "children": [
                  {"name": "aaa",
                   "key": 1,
                   "children": [
                       {"name": "bbb", "key": 2},
                       {"name": "ccc", "key": 3},
                   ]},
                  {"name": "ddd", "key": 4, "children": []}
              ]};

class FoldIndicator extends React.Component
{
    constructor(props)
    {
        super(props);
        this.state = {"folded": props["folded"]};
    }

    onclick = (e) => {
        this.setState({"folded": !this.state["folded"]});
        this.props["onClick"]();
    }

    render()
    {
        let transform = "";
        if(this.state["folded"])
        {
            transform = "translate(0, 8) rotate(-90)";
        }

        return e("span", {"onClick": this.onclick},
                 e("svg", {"version": "1.1",
                           "width": 8, "height": 8,
                           "xmlns": "http://www.w3.org/2000/svg",
                          },
                 e("polygon", {"points": "0,2 8,2 4,6", "fill": "black",
                               "transform": transform})));
    }
}

class TreeItem extends React.Component
{
    constructor(props)
    {
        super(props);
        if(props["folded"] === undefined)
        {
            this.state = {"folded": false};
        }
        else
        {
            this.state = {"folded": props["folded"]};
        }
    }

    render()
    {
        let selection = this.props["getSelection"]();
        console.log(`Drawing item ${this.props["data"]["key"]}, selection is ${selection}, selected is ${this.props["selected"]}`);
        let children = [];
        if(!this.state["folded"])
        {
            if(this.props["data"]["children"] !== undefined)
            {
                for(let i in this.props["data"]["children"])
                {
                    let child = this.props["data"]["children"][i];
                    let props = {"data": child, "key": child["key"],
                                 "depth": this.props["depth"] + 1,
                                 "selected": selection === child["key"],
                                 "onSelect": this.props["onSelect"],
                                 "getSelection": this.props["getSelection"],
                                };
                    let element = e(TreeItem, props);
                    children.push(element);
                }
            }
        }

        let class_name = "";
        if(this.props["selected"])
        {
            class_name = "TreeItemSelected";
        }
        return e("div", null,
                 e("div", {"style": {"marginLeft": this.props["depth"] * 16}},
                   e(FoldIndicator, {"onClick": () => {
                       this.setState({"folded": !this.state["folded"]});
                   }}),
                   e("span", {"className": class_name,
                              "onClick": () => {
                                  this.props["onSelect"](this.props["data"]["key"]);
                              }},
                     this.props["data"]["name"])),
                 children);
    }
}

class TreeView extends React.Component
{
    constructor(props)
    {
        super(props);
        this.state = {"selected": props["selection"]};
        this.onSelect = this.onSelect.bind(this);
        this.getSelection = this.getSelection.bind(this);
    }

    onSelect(key)
    {
        console.log(`Selected item ${key}.`);
        this.setState({"selected": key});
    }

    getSelection()
    {
        return this.state["selected"];
    }

    render()
    {
        return e(TreeItem, {"data": this.props["data"], "depth": 0,
                            "selected": this.getSelection() === this.props["data"]["key"],
                            "onSelect": this.onSelect,
                            "getSelection": this.getSelection,
                           });
    }
}

const domContainer = document.querySelector('#Main');
ReactDOM.render(e(TreeView, {"data": data, "selection": 0}), domContainer);
