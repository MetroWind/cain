// Properties: label, onClick, id
class Button extends React.Component
{
    constructor(props)
    {
        super(props);
        this.state = {};
    }

    render()
    {
        return e("a", {"className": "Button", "id": this.props["id"],
                       "href": "#", "onclick": this.props["onClick"]},
                 this.props["label"]);
    }
}
