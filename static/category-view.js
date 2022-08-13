function categoryName(category)
{
    return category["data"]["name"];
}

function categoryID(category)
{
    return category["data"]["id"];
}

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
        let children = [];
        if(!this.state["folded"])
        {
            if(this.props["data"]["children"] !== undefined)
            {
                for(let i in this.props["data"]["children"])
                {
                    let child = this.props["data"]["children"][i];
                    let props = {"data": child, "key": categoryID(child),
                                 "depth": this.props["depth"] + 1,
                                 "selected": selection === categoryID(child),
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
                                  this.props["onSelect"](categoryID(this.props["data"]));
                              }},
                     categoryName(this.props["data"]))),
                 children);
    }
}

// Props: data, onSelect, getSelection
class TreeView extends React.Component
{
    constructor(props)
    {
        super(props);
        this.state = {"selected": props["selection"]};
        this.onSelect = this.onSelect.bind(this);
        this.getSelection = this.getSelection.bind(this);
    }

    render()
    {
        return e(TreeItem, {"data": this.props["data"], "depth": 0,
                            "selected": this.props["getSelection"]() ===
                                categoryID(this.props["data"]),
                            "onSelect": this.props["onSelect"],
                            "getSelection": this.props["getSelection"],
                           });
    }
}
