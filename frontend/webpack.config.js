var webpack = require('webpack')
var MinifyPlugin = require("babel-minify-webpack-plugin");
var DEV = process.env['NODE_ENV'] != 'production';
module.exports = {
    context: __dirname,
    entry: "./index",
    output: {
        path: (process.env.OUTPUT_DIR || (__dirname + "/../target")) + "/js",
        filename: "bundle.js",
        publicPath: '.',
    },
    module: {
        loaders: [{
            test: /\.khufu$/,
            loaders: ['babel-loader', 'khufu'],
            exclude: /node_modules/,
        }, {
            test: /\.js$/,
            loaders: ['babel-loader'],
            exclude: /node_modules/,
        }],
    },
    resolve: {
        extensions: ['.js', '.khufu'],
        modules: process.env.NODE_PATH.split(':').filter(x => x),
    },
    resolveLoader: {
        mainFields: ["webpackLoader", "main", "browser"],
        modules: process.env.NODE_PATH.split(':').filter(x => x),
    },
    plugins: [
        new webpack.LoaderOptionsPlugin({
            options: {
                khufu: {
                    static_attrs: !DEV,
                },
                babel: {
                    "plugins": [
                        "transform-strict-mode",
                        "transform-object-rest-spread",
                    ],
                    "env": {
                        "production": {
                            "presets": ["minify"],
                        },
                    },
                },
            }
        }),
        new webpack.NoEmitOnErrorsPlugin(),
        new webpack.DefinePlugin({
            VERSION: JSON.stringify("v0.4.1"),
            "process.env.NODE_ENV": JSON.stringify(process.env['NODE_ENV']),
            DEBUG: DEV,
        }),
    ].concat(DEV ? [] : [
        new MinifyPlugin({}, {}),
    ]),
}

