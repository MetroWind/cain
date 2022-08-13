class EntryItemView extends React.Component
{
    constructor(props)
    {
        super(props);
    }

    render()
    {
        return e("li", null,
                 e("a", {"href": "#"}, this.props["title"]));
    }
}

class EntryList extends React.Component
{
    constructor(props)
    {
        super(props);
    }

    render()
    {
        let entry_views = this.props["entries"].map(entry =>
            e(EntryItemView, {"entry": entry, "key": entry["id"]}));
        return e("ul", null, entry_views);
    }
}

class EntryListPane extends React.Component
{
    constructor(props)
    {
        super(props);
        this.state = {};
    }
    render()
    {
        return e("div", null,
                 e("div", null, "Entries"),
                 e(EntryList, {"entries": this.props["entries"]})),
    }
}

// Props: onClickAddCategory, onSelectCategory, getSelection, loadCategories
class CategoryPane extends React.Component
{
    constructor(props)
    {
        super(props);
    }

    render()
    {
        let categories = this.props["loadCategories"]();
        return e("div", null,
                 e("div", null,
                   e("span", null, "Categories"),
                   e(Button, {"label": "+", "onClick": this.props["onClickAddCategory"]})),
                 e(TreeView, {"data": categories,
                              "onSelect": this.props["onSelectCategory"],
                              "getSelection": this.props["getSelection"],
                             }),
    }
}

class MainView extends React.Component
{
    constructor(props)
    {
        super(props);
        this.state = {
            "selected_category": 0,
        };
        this.onClickAddCategory = this.onClickAddCategory.bind(this);
        this.onSelectCategory = this.onSelectCategory.bind(this);
        this.getCategorySelection = this.getCategorySelection.bind(this);
        this.loadCategories = this.loadCategories.bind(this);
    }

    onClickAddCategory()
    {
    }

    onSelectCategory(key)
    {
        this.setState({"selected_category": key});
    }

    getCategorySelection()
    {
        return this.state["selected_category"];
    }

    loadCategories()
    {
    }

    render()
    {
        return e("div", null,
                 e(CategoryPane, {"loadCategories": this.loadCategories,
                                  "onClickAddCategory": this.onClickAddCategory,
                                  "onSelectCategory": this.onSelectCategory,
                                  "getSelection": this.getCategorySelection,
                                 }),
                 e(EntryListPane, {"entries": this.props["entries"],
                                  }));
    }
}
