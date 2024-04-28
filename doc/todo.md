# File System

## update file list from local agent

* load file list on startup
* load changes, indicated by change number
    - load all list items if change number is newer
    - or load all the changed list items only (requires to maintain a detailed change record)
* requires agent to maintain a change number
* has to return a change number as version number

## get file data from local agent

Using TCP protocol if the Agent is somwhere else.

## organize files by tag locally

only the files and their IDs are distributed on the agents.



# Agent

## maintain files in a set of directories

* Monitor the directories for change
* add own information if changed by some client job

## update file list from other agent

* maintain local change number
* maintain dependency for local change number on remote change numbers
* agent-id + change number are unique
* change numbers need to be strictly increasing numbers


# Client




# Change Watermark

Tells exactly wich change level an agent has. And so a client can understand how far behind it might be.